//! `karoowa node` — start a Karoowa node.

use clap::Args;
use karoowa_api::server::{start_server, ServerConfig};
use karoowa_consensus::{BlockProducer, PoAConfig, PoAEngine, ProducerConfig};
use karoowa_core::{Account, BlockHeader};
use karoowa_crypto::{Address, Hash, Keypair};
use karoowa_network::{Network, NetworkConfig};
use karoowa_storage::{BlockStore, RocksStorage, StateStore};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Args)]
pub struct NodeArgs {
    /// Path to the validator key file (hex-encoded private key)
    #[arg(long)]
    validator_key: PathBuf,

    /// Consensus engine to use
    #[arg(long, default_value = "poa")]
    consensus: String,

    /// Data directory for RocksDB
    #[arg(long, default_value = "./data")]
    data_dir: PathBuf,

    /// Bootnode multiaddresses (comma-separated)
    #[arg(long, value_delimiter = ',')]
    bootnodes: Vec<String>,

    /// RPC listen port
    #[arg(long, default_value = "8545")]
    rpc_port: u16,

    /// Address the RPC binds to.
    ///
    /// Defaults to `0.0.0.0`, which is what the node has always done — changing
    /// it would break every existing deployment on upgrade. But the RPC is
    /// **unauthenticated**, so a non-loopback bind hands node control and
    /// mempool submission to anyone who can reach the host. Pass
    /// `--rpc-bind 127.0.0.1` unless it is firewalled or sits behind an
    /// authenticating proxy; the node warns at startup when it is not.
    #[arg(long, default_value = "0.0.0.0")]
    rpc_bind: std::net::IpAddr,

    /// P2P listen port
    #[arg(long, default_value = "30303")]
    p2p_port: u16,

    /// Block production interval in seconds
    #[arg(long, default_value = "2")]
    block_time: u64,

    /// License file (no-op in M1 — reserved for enterprise features)
    #[arg(long)]
    license_file: Option<PathBuf>,

    /// Join a named network (e.g. "public-devnet")
    #[arg(long)]
    join: Option<String>,

    /// Seed a starting balance on a fresh chain. Format: `<address>=<amount>`.
    /// Repeatable. Applied only when no existing chain data is present — does
    /// nothing on a resumed node. The address is 0x-prefixed hex, the amount
    /// is a raw u64 in the chain's smallest unit.
    ///
    /// Example:
    ///   --genesis-allocation 0xdeadbeef...=1000000000000000000
    #[arg(long = "genesis-allocation", value_name = "ADDR=AMOUNT")]
    genesis_allocations: Vec<String>,
}

pub async fn run(args: NodeArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Load validator key. Name the path and, on a permission error, say what to
    // do about it — key files are 0600, so "wrong user" is the likeliest cause
    // and a bare `Permission denied (os error 13)` gives the operator nothing.
    let key_hex = std::fs::read_to_string(&args.validator_key)
        .map_err(|e| {
            let path = args.validator_key.display();
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                format!(
                    "cannot read validator key {path}: {e}\n\
                     Key files are written 0600, readable only by their owner. \
                     Check that this process runs as the user that owns the key \
                     (`ls -l {path}`), or hand it over with `chown`. In a \
                     container the key must be readable by uid 65532."
                )
            } else {
                format!("cannot read validator key {path}: {e}")
            }
        })?
        .trim()
        .to_string();
    let key_bytes = hex::decode(key_hex.strip_prefix("0x").unwrap_or(&key_hex))?;
    if key_bytes.len() != 32 {
        return Err(format!("validator key must be 32 bytes, got {}", key_bytes.len()).into());
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&key_bytes);
    let keypair = Keypair::from_seed(&seed);
    let validator_address = keypair.address();

    info!(address = %validator_address, "loaded validator key");

    // Open storage.
    std::fs::create_dir_all(&args.data_dir)?;
    let storage = Arc::new(RocksStorage::open(&args.data_dir)?);

    // Start P2P network.
    let mut bootnodes: Vec<libp2p::Multiaddr> = args
        .bootnodes
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    if let Some(ref network_name) = args.join {
        if network_name == "public-devnet" {
            // Public devnet bootnode (AWS Lightsail, eu-west-2).
            let devnet_bootnode: libp2p::Multiaddr =
                "/ip4/18.171.150.73/tcp/30303".parse().unwrap();
            bootnodes.push(devnet_bootnode);
            info!("joining public devnet — bootnode at 18.171.150.73:30303");
        }
    }

    let net_config = NetworkConfig {
        listen_addr: format!("/ip4/0.0.0.0/tcp/{}", args.p2p_port).parse()?,
        bootnodes,
        keypair_seed: None,
        ..NetworkConfig::default()
    };
    let network = Network::start(net_config).await?;

    info!(peer_id = %network.local_peer_id(), "P2P network started");

    // Start API server.
    let server_config = ServerConfig {
        bind_addr: SocketAddr::new(args.rpc_bind, args.rpc_port),
        chain_id: 1,
    };
    if !args.rpc_bind.is_loopback() {
        warn!(
            bind = %args.rpc_bind,
            port = args.rpc_port,
            "RPC is listening on a non-loopback address and is UNAUTHENTICATED. \
             Anyone who can reach this host can control the node and submit \
             transactions. Pass --rpc-bind 127.0.0.1, or firewall the port / \
             front it with an authenticating proxy."
        );
    }
    let (api_addr, _mempool, _subscriptions, _api_handle) =
        start_server(server_config, Arc::clone(&storage), network.clone()).await?;

    info!(addr = %api_addr, "API server started");

    // Set up consensus.
    match args.consensus.as_str() {
        "poa" => {
            let poa_config = PoAConfig {
                validators: vec![validator_address],
            };
            let engine = Arc::new(PoAEngine::new(poa_config)?);

            let genesis_header = match storage.head()? {
                Some(block) => {
                    info!(height = block.height(), "resuming from existing chain");
                    block.header
                }
                None => {
                    info!("no existing chain, starting from genesis");
                    apply_genesis_allocations(&storage, &args.genesis_allocations)?;
                    BlockHeader {
                        parent_hash: Hash::ZERO,
                        state_root: Hash::ZERO,
                        tx_root: Hash::ZERO,
                        receipt_root: Hash::ZERO,
                        height: 0,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                        proposer: validator_address,
                        consensus_data: vec![],
                    }
                }
            };

            let producer_config = ProducerConfig {
                validator_keypair: Arc::new(keypair),
                block_interval: Duration::from_secs(args.block_time),
            };

            let (producer, mut block_rx) =
                BlockProducer::new(engine, producer_config, genesis_header);

            let storage_handle = Arc::clone(&storage);
            let network_handle = network.clone();
            tokio::spawn(async move {
                while let Some(block) = block_rx.recv().await {
                    if let Err(e) = storage_handle.put_block(&block) {
                        tracing::error!(error = %e, "failed to persist block");
                        continue;
                    }
                    if let Err(e) = network_handle.broadcast_block(&block).await {
                        tracing::warn!(error = %e, "failed to broadcast block");
                    }
                }
            });

            info!(
                engine = "poa",
                block_time = args.block_time,
                "starting block production"
            );
            producer.run().await?;
        }
        other => {
            return Err(format!("unknown consensus engine: {other}. Available: poa").into());
        }
    }

    Ok(())
}

/// Apply `--genesis-allocation ADDR=AMOUNT` entries to a fresh state.
/// Called exactly once at node startup when there is no existing chain data.
fn apply_genesis_allocations(
    storage: &RocksStorage,
    allocations: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if allocations.is_empty() {
        return Ok(());
    }
    for entry in allocations {
        let (addr_str, amount_str) = entry.split_once('=').ok_or_else(|| {
            format!("invalid --genesis-allocation (expected ADDR=AMOUNT): {entry}")
        })?;
        let address: Address = addr_str
            .parse()
            .map_err(|e| format!("invalid address in --genesis-allocation {entry}: {e:?}"))?;
        let amount: u64 = amount_str
            .parse()
            .map_err(|e| format!("invalid amount in --genesis-allocation {entry}: {e}"))?;

        let account = Account {
            nonce: 0,
            balance: amount,
            code_hash: Hash::ZERO,
            storage_root: Hash::ZERO,
        };
        storage.put_account(&address, &account)?;
        info!(
            address = %address,
            amount = amount,
            "genesis allocation applied"
        );
    }
    Ok(())
}

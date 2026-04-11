//! `karoowa client` — quick one-shot RPC queries.

use clap::{Args, Subcommand};
use karoowa_sdk::NodeClient;

#[derive(Args)]
pub struct ClientArgs {
    /// RPC endpoint URL
    #[arg(long, default_value = "http://localhost:8545")]
    rpc: String,

    #[command(subcommand)]
    command: ClientCommand,
}

#[derive(Subcommand)]
enum ClientCommand {
    /// Get the current block height
    BlockNumber,

    /// Get the chain ID
    ChainId,

    /// Get the balance of an address
    GetBalance {
        /// Address (0x-prefixed hex)
        address: String,
    },

    /// Get the nonce of an address
    GetNonce {
        /// Address (0x-prefixed hex)
        address: String,
    },

    /// Get node info
    NodeInfo,

    /// Get connected peer count
    PeerCount,

    /// Check sync status
    Syncing,
}

pub async fn run(args: ClientArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = NodeClient::new(&args.rpc);

    match args.command {
        ClientCommand::BlockNumber => {
            let height = client.block_number().await?;
            println!("{height}");
        }
        ClientCommand::ChainId => {
            let id = client.chain_id().await?;
            println!("{id}");
        }
        ClientCommand::GetBalance { address } => {
            let addr = address
                .parse()
                .map_err(|e| format!("invalid address: {e}"))?;
            let balance = client.get_balance(&addr).await?;
            println!("{balance}");
        }
        ClientCommand::GetNonce { address } => {
            let addr = address
                .parse()
                .map_err(|e| format!("invalid address: {e}"))?;
            let nonce = client.get_transaction_count(&addr).await?;
            println!("{nonce}");
        }
        ClientCommand::NodeInfo => {
            let info = client.node_info().await?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        ClientCommand::PeerCount => {
            let count = client.peer_count().await?;
            println!("{count}");
        }
        ClientCommand::Syncing => {
            let syncing = client.syncing().await?;
            println!("{syncing}");
        }
    }

    Ok(())
}

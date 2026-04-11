//! Integration tests for the Karoowa SDK.
//!
//! Each test starts a real API server backed by RocksDB + libp2p, then
//! exercises the SDK against it.

use karoowa_api::server::{start_server, ServerConfig};
use karoowa_core::*;
use karoowa_crypto::*;
use karoowa_network::{Network, NetworkConfig};
use karoowa_sdk::{NodeClient, TransferBuilder, Wallet};
use karoowa_storage::{BlockStore, RocksStorage, StateStore};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;

async fn start_test_node() -> (String, TempDir, Arc<RocksStorage>) {
    let dir = TempDir::new().unwrap();
    let storage = Arc::new(RocksStorage::open(dir.path()).unwrap());

    let net_config = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        keypair_seed: Some([50; 32]),
        ..NetworkConfig::default()
    };
    let network = Network::start(net_config).await.unwrap();

    let server_config = ServerConfig {
        bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        chain_id: 42,
    };

    let (addr, _handle) = start_server(server_config, Arc::clone(&storage), network)
        .await
        .unwrap();

    (format!("http://{addr}"), dir, storage)
}

#[tokio::test]
async fn client_chain_id() {
    let (url, _dir, _storage) = start_test_node().await;
    let client = NodeClient::new(&url);
    assert_eq!(client.chain_id().await.unwrap(), 42);
}

#[tokio::test]
async fn client_block_number() {
    let (url, _dir, storage) = start_test_node().await;
    let client = NodeClient::new(&url);

    assert_eq!(client.block_number().await.unwrap(), 0);

    // Store a block.
    let proposer = Address::from_public_key(&[1u8; 32]);
    let block = BlockBuilder::new(Hash::ZERO, 0, 1700000000, proposer).build();
    storage.put_block(&block).unwrap();

    assert_eq!(client.block_number().await.unwrap(), 0); // height 0 is the first block
}

#[tokio::test]
async fn client_get_balance() {
    let (url, _dir, storage) = start_test_node().await;
    let client = NodeClient::new(&url);

    let addr = Address::from_public_key(&[10u8; 32]);
    assert_eq!(client.get_balance(&addr).await.unwrap(), 0);

    storage
        .put_account(
            &addr,
            &Account {
                balance: 5000,
                nonce: 0,
                ..Account::default()
            },
        )
        .unwrap();

    assert_eq!(client.get_balance(&addr).await.unwrap(), 5000);
}

#[tokio::test]
async fn client_get_transaction_count() {
    let (url, _dir, storage) = start_test_node().await;
    let client = NodeClient::new(&url);

    let addr = Address::from_public_key(&[11u8; 32]);
    storage
        .put_account(
            &addr,
            &Account {
                balance: 1000,
                nonce: 7,
                ..Account::default()
            },
        )
        .unwrap();

    assert_eq!(client.get_transaction_count(&addr).await.unwrap(), 7);
}

#[tokio::test]
async fn client_syncing() {
    let (url, _dir, _storage) = start_test_node().await;
    let client = NodeClient::new(&url);
    assert!(!client.syncing().await.unwrap());
}

#[tokio::test]
async fn client_peer_count() {
    let (url, _dir, _storage) = start_test_node().await;
    let client = NodeClient::new(&url);
    assert_eq!(client.peer_count().await.unwrap(), 0);
}

#[tokio::test]
async fn client_node_info() {
    let (url, _dir, _storage) = start_test_node().await;
    let client = NodeClient::new(&url);
    let info = client.node_info().await.unwrap();
    assert_eq!(info["chain_id"], 42);
    assert_eq!(info["name"], "karoowa");
}

#[tokio::test]
async fn client_send_raw_transaction() {
    let (url, _dir, _storage) = start_test_node().await;
    let client = NodeClient::new(&url);

    let wallet = Wallet::from_seed(&[1u8; 32], 42);
    let to = Address::from_public_key(&[2u8; 32]);
    let tx = wallet.sign_transfer(to, 100, 0, 1, 21000);
    let hex = Wallet::encode_transaction(&tx).unwrap();

    let tx_hash = client.send_raw_transaction(&hex).await.unwrap();
    assert!(!tx_hash.is_empty());

    // Should appear in pending.
    let pending = client.pending_transactions().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0], tx_hash);
}

#[tokio::test]
async fn wallet_and_transfer_builder() {
    let (url, _dir, _storage) = start_test_node().await;
    let client = NodeClient::new(&url);

    let wallet = Wallet::from_seed(&[5u8; 32], 42);
    let to = Address::from_public_key(&[6u8; 32]);

    let tx = TransferBuilder::new()
        .to(to)
        .value(250)
        .nonce(0)
        .gas_price(1)
        .gas_limit(21_000)
        .sign(&wallet);

    let hex = Wallet::encode_transaction(&tx).unwrap();
    let tx_hash = client.send_raw_transaction(&hex).await.unwrap();
    assert!(!tx_hash.is_empty());
}

#[tokio::test]
async fn client_unknown_method_returns_rpc_error() {
    let (url, _dir, _storage) = start_test_node().await;

    // Directly call a non-existent method via the raw HTTP client.
    let http = reqwest::Client::new();
    let resp: serde_json::Value = http
        .post(format!("{url}/rpc"))
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "kw_doesNotExist",
            "params": [],
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp["error"].is_object());
}

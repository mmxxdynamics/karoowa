//! Integration tests for the Karoowa API gateway.
//!
//! Each test starts a real API server backed by RocksDB (in a temp dir)
//! and a real libp2p network node, then hits the endpoints with reqwest.

use karoowa_api::server::{start_server, ServerConfig};
use karoowa_core::*;
use karoowa_crypto::*;
use karoowa_network::{Network, NetworkConfig};
use karoowa_storage::{BlockStore, RocksStorage, StateStore};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;

/// Start a test server, returning the URL and temp dir (kept alive for the
/// test duration).
async fn start_test_server() -> (String, TempDir, Arc<RocksStorage>) {
    let dir = TempDir::new().unwrap();
    let storage = Arc::new(RocksStorage::open(dir.path()).unwrap());

    let net_config = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        keypair_seed: Some([42; 32]),
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

    let url = format!("http://{addr}");
    (url, dir, storage)
}

fn rpc_request(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    })
}

// ---------------------------------------------------------------------------
// Health endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_returns_ok() {
    let (url, _dir, _storage) = start_test_server().await;
    let resp = reqwest::get(format!("{url}/health")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

// ---------------------------------------------------------------------------
// JSON-RPC: kw_chainId
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_chain_id() {
    let (url, _dir, _storage) = start_test_server().await;
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_chainId", json!([])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"], 42);
}

// ---------------------------------------------------------------------------
// JSON-RPC: kw_blockNumber
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_block_number_empty_chain() {
    let (url, _dir, _storage) = start_test_server().await;
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_blockNumber", json!([])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"], 0);
}

#[tokio::test]
async fn rpc_block_number_after_storing_blocks() {
    let (url, _dir, storage) = start_test_server().await;

    // Store a few blocks directly.
    let kp = Keypair::from_seed(&[1u8; 32]);
    let to = Address::from_public_key(&[2u8; 32]);
    let proposer = Address::from_public_key(&[99u8; 32]);

    let mut parent = Hash::ZERO;
    for i in 0..5u64 {
        let tx = Transaction::sign_transfer(&kp, to, 100, i, 1, 21000, 1);
        let block = BlockBuilder::new(parent, i, 1700000000 + i, proposer)
            .transactions(vec![tx])
            .build();
        parent = block.hash();
        storage.put_block(&block).unwrap();
    }

    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_blockNumber", json!([])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"], 4);
}

// ---------------------------------------------------------------------------
// JSON-RPC: kw_getBalance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_get_balance() {
    let (url, _dir, storage) = start_test_server().await;

    let addr = Address::from_public_key(&[1u8; 32]);
    let account = Account {
        balance: 999_999,
        nonce: 0,
        ..Account::default()
    };
    storage.put_account(&addr, &account).unwrap();

    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_getBalance", json!([addr.to_string()])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"], 999_999);
}

#[tokio::test]
async fn rpc_get_balance_unknown_address() {
    let (url, _dir, _storage) = start_test_server().await;

    let addr = Address::from_public_key(&[77u8; 32]);
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_getBalance", json!([addr.to_string()])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"], 0);
}

// ---------------------------------------------------------------------------
// JSON-RPC: kw_peerCount
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_peer_count() {
    let (url, _dir, _storage) = start_test_server().await;
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_peerCount", json!([])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"], 0); // no peers in test
}

// ---------------------------------------------------------------------------
// JSON-RPC: kw_nodeInfo
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_node_info() {
    let (url, _dir, _storage) = start_test_server().await;
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_nodeInfo", json!([])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"]["chain_id"], 42);
    assert_eq!(resp["result"]["name"], "karoowa");
    assert!(resp["result"]["peer_id"].is_string());
}

// ---------------------------------------------------------------------------
// JSON-RPC: kw_syncing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_syncing() {
    let (url, _dir, _storage) = start_test_server().await;
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_syncing", json!([])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"], false);
}

// ---------------------------------------------------------------------------
// JSON-RPC: unknown method
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_unknown_method() {
    let (url, _dir, _storage) = start_test_server().await;
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/rpc"))
        .json(&rpc_request("kw_doesNotExist", json!([])))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], -32601);
}

// ---------------------------------------------------------------------------
// REST: /api/v1/status
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rest_status() {
    let (url, _dir, _storage) = start_test_server().await;
    let resp: Value = reqwest::get(format!("{url}/api/v1/status"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["chain_id"], 42);
    assert!(resp["block_height"].is_number());
    assert!(resp["peer_count"].is_number());
}

// ---------------------------------------------------------------------------
// REST: /api/v1/blocks/:height
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rest_block_by_height() {
    let (url, _dir, storage) = start_test_server().await;

    let proposer = Address::from_public_key(&[1u8; 32]);
    let block = BlockBuilder::new(Hash::ZERO, 0, 1700000000, proposer).build();
    storage.put_block(&block).unwrap();

    let resp = reqwest::get(format!("{url}/api/v1/blocks/0"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn rest_block_not_found() {
    let (url, _dir, _storage) = start_test_server().await;
    let resp = reqwest::get(format!("{url}/api/v1/blocks/999"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

// ---------------------------------------------------------------------------
// REST: /api/v1/account/:address
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rest_account() {
    let (url, _dir, storage) = start_test_server().await;

    let addr = Address::from_public_key(&[5u8; 32]);
    let account = Account {
        balance: 42_000,
        nonce: 7,
        ..Account::default()
    };
    storage.put_account(&addr, &account).unwrap();

    let resp: Value = reqwest::get(format!("{url}/api/v1/account/{addr}"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["balance"], 42_000);
    assert_eq!(resp["nonce"], 7);
}

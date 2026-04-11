//! Integration test: two in-process Karoowa nodes exchange blocks and
//! transactions over libp2p.

use karoowa_core::{Block, BlockBuilder, Transaction};
use karoowa_crypto::{Address, Hash, Keypair};
use karoowa_network::{Network, NetworkConfig};
use std::time::Duration;

fn test_keypair(seed: u8) -> Keypair {
    Keypair::from_seed(&[seed; 32])
}

fn make_tx(kp: &Keypair, nonce: u64) -> Transaction {
    let to = Address::from_public_key(&[99u8; 32]);
    Transaction::sign_transfer(kp, to, 100, nonce, 1, 21000, 1)
}

fn make_block(height: u64, parent: Hash, txs: Vec<Transaction>) -> Block {
    let proposer = Address::from_public_key(&[1u8; 32]);
    BlockBuilder::new(parent, height, 1700000000 + height, proposer)
        .transactions(txs)
        .build()
}

/// Helper: start two nodes and connect them.
async fn start_connected_pair() -> (
    karoowa_network::NetworkHandle,
    karoowa_network::NetworkHandle,
) {
    let config_a = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: vec![],
        keypair_seed: Some([1; 32]),
        ..NetworkConfig::default()
    };

    let handle_a = Network::start(config_a).await.unwrap();

    // Wait for node A to be listening and get its address.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let addrs_a = handle_a.listen_addresses().await.unwrap();
    assert!(!addrs_a.is_empty(), "node A should be listening");

    // Node B connects to node A via its listen address.
    let config_b = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: addrs_a,
        keypair_seed: Some([2; 32]),
        ..NetworkConfig::default()
    };

    let handle_b = Network::start(config_b).await.unwrap();

    // Wait for the connection to establish and Gossipsub mesh to form.
    // Gossipsub needs a few heartbeat intervals to graft peers.
    tokio::time::sleep(Duration::from_secs(3)).await;

    (handle_a, handle_b)
}

#[tokio::test]
async fn two_nodes_connect() {
    let (handle_a, handle_b) = start_connected_pair().await;

    assert_eq!(handle_a.peer_count(), 1, "node A should have 1 peer");
    assert_eq!(handle_b.peer_count(), 1, "node B should have 1 peer");

    // Verify they see each other's PeerId.
    let peers_a = handle_a.connected_peers().await.unwrap();
    let peers_b = handle_b.connected_peers().await.unwrap();
    assert!(peers_a.contains(&handle_b.local_peer_id()));
    assert!(peers_b.contains(&handle_a.local_peer_id()));
}

#[tokio::test]
async fn broadcast_block_is_received() {
    let (handle_a, handle_b) = start_connected_pair().await;

    // Node B subscribes to blocks.
    let mut block_rx = handle_b.subscribe_blocks();

    // Node A broadcasts a block.
    let kp = test_keypair(1);
    let block = make_block(1, Hash::ZERO, vec![make_tx(&kp, 0)]);
    let expected_hash = block.hash();

    handle_a.broadcast_block(&block).await.unwrap();

    // Node B should receive the block within 2 seconds.
    let received = tokio::time::timeout(Duration::from_secs(2), block_rx.recv())
        .await
        .expect("timed out waiting for block")
        .expect("channel error");

    assert_eq!(received.hash(), expected_hash);
    assert_eq!(received.height(), 1);
    assert_eq!(received.transactions.len(), 1);
}

#[tokio::test]
async fn broadcast_transaction_is_received() {
    let (handle_a, handle_b) = start_connected_pair().await;

    let mut tx_rx = handle_b.subscribe_transactions();

    let kp = test_keypair(1);
    let tx = make_tx(&kp, 42);
    let expected_hash = tx.hash();

    handle_a.broadcast_transaction(&tx).await.unwrap();

    let received = tokio::time::timeout(Duration::from_secs(2), tx_rx.recv())
        .await
        .expect("timed out waiting for transaction")
        .expect("channel error");

    assert_eq!(received.hash(), expected_hash);
}

#[tokio::test]
async fn broadcast_multiple_blocks_in_sequence() {
    let (handle_a, handle_b) = start_connected_pair().await;

    let mut block_rx = handle_b.subscribe_blocks();
    let kp = test_keypair(1);

    let mut parent = Hash::ZERO;
    for i in 0..5u64 {
        let block = make_block(i, parent, vec![make_tx(&kp, i)]);
        parent = block.hash();
        handle_a.broadcast_block(&block).await.unwrap();
    }

    // Receive all 5 blocks.
    for i in 0..5u64 {
        let received = tokio::time::timeout(Duration::from_secs(2), block_rx.recv())
            .await
            .expect("timed out waiting for block")
            .expect("channel error");
        assert_eq!(received.height(), i);
    }
}

#[tokio::test]
async fn peer_count_updates_on_disconnect() {
    let config_a = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: vec![],
        keypair_seed: Some([10; 32]),
        ..NetworkConfig::default()
    };
    let handle_a = Network::start(config_a).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let addrs_a = handle_a.listen_addresses().await.unwrap();

    assert_eq!(handle_a.peer_count(), 0);

    // Start node B and connect.
    let config_b = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: addrs_a,
        keypair_seed: Some([11; 32]),
        ..NetworkConfig::default()
    };
    let handle_b = Network::start(config_b).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;

    assert_eq!(handle_a.peer_count(), 1);

    // Drop handle_b — its event loop stops, which closes the connection.
    drop(handle_b);
    tokio::time::sleep(Duration::from_secs(2)).await;

    assert_eq!(handle_a.peer_count(), 0);
}

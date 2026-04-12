//! Integration test: two in-process Karoowa nodes exchange snapshots
//! over the libp2p `/karoowa/state-sync/1` request-response protocol.

use async_trait::async_trait;
use karoowa_core::Account;
use karoowa_crypto::{sha3_256, Address};
use karoowa_network::{
    Network, NetworkConfig, SnapshotProvider, SnapshotRequest, SnapshotResponse,
};
use karoowa_storage::{
    InMemorySnapshotStore, SnapshotChunk, SnapshotEntry, SnapshotManifest, SnapshotStore,
};
use std::sync::Arc;
use std::time::Duration;

/// Adapter that wires `InMemorySnapshotStore` to the network's `SnapshotProvider` trait.
struct StoreProvider {
    store: Arc<InMemorySnapshotStore>,
}

#[async_trait]
impl SnapshotProvider for StoreProvider {
    async fn list_snapshots(&self) -> Vec<SnapshotManifest> {
        self.store.list_snapshots().unwrap_or_default()
    }

    async fn get_manifest(&self, height: u64) -> Option<SnapshotManifest> {
        self.store.get_manifest(height).ok().flatten()
    }

    async fn get_chunk(&self, height: u64, index: u32) -> Option<SnapshotChunk> {
        self.store.get_chunk(height, index).ok().flatten()
    }
}

fn make_entry(seed: u8, balance: u64) -> SnapshotEntry {
    SnapshotEntry {
        address: Address::from_public_key(&[seed; 32]),
        account: Account {
            balance,
            nonce: 0,
            ..Account::default()
        },
    }
}

async fn start_pair_with_provider(
    provider_store: Arc<InMemorySnapshotStore>,
) -> (
    karoowa_network::NetworkHandle,
    karoowa_network::NetworkHandle,
) {
    // Server node A — has the snapshot provider.
    let config_a = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: vec![],
        keypair_seed: Some([1; 32]),
        ..NetworkConfig::default()
    };
    let handle_a = Network::start(config_a).await.unwrap();
    handle_a
        .set_snapshot_provider(Arc::new(StoreProvider {
            store: provider_store,
        }))
        .await
        .unwrap();

    // Wait for A to be listening, then get its address.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let addrs_a = handle_a.listen_addresses().await.unwrap();
    assert!(!addrs_a.is_empty(), "node A should be listening");

    // Client node B — connects to A.
    let config_b = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: addrs_a,
        keypair_seed: Some([2; 32]),
        ..NetworkConfig::default()
    };
    let handle_b = Network::start(config_b).await.unwrap();

    // Wait for connection.
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(handle_a.peer_count(), 1, "node A should see 1 peer");
    assert_eq!(handle_b.peer_count(), 1, "node B should see 1 peer");

    (handle_a, handle_b)
}

#[tokio::test]
async fn list_snapshots_over_network() {
    let store = Arc::new(InMemorySnapshotStore::new());
    store
        .create_snapshot(
            10,
            sha3_256(b"state-at-10"),
            vec![make_entry(1, 100), make_entry(2, 200)],
        )
        .unwrap();
    store
        .create_snapshot(20, sha3_256(b"state-at-20"), vec![make_entry(3, 300)])
        .unwrap();

    let (handle_a, handle_b) = start_pair_with_provider(Arc::clone(&store)).await;

    let response = handle_b
        .request_snapshot(handle_a.local_peer_id(), SnapshotRequest::ListSnapshots)
        .await
        .unwrap();

    match response {
        SnapshotResponse::Manifests(manifests) => {
            assert_eq!(manifests.len(), 2);
        }
        other => panic!("expected Manifests, got {other:?}"),
    }
}

#[tokio::test]
async fn get_manifest_over_network() {
    let store = Arc::new(InMemorySnapshotStore::new());
    let original_manifest = store
        .create_snapshot(42, sha3_256(b"state-42"), vec![make_entry(1, 1000)])
        .unwrap();

    let (handle_a, handle_b) = start_pair_with_provider(Arc::clone(&store)).await;

    let response = handle_b
        .request_snapshot(
            handle_a.local_peer_id(),
            SnapshotRequest::GetManifest { height: 42 },
        )
        .await
        .unwrap();

    match response {
        SnapshotResponse::Manifest(Some(m)) => {
            assert_eq!(m, original_manifest);
        }
        other => panic!("expected Manifest(Some(_)), got {other:?}"),
    }
}

#[tokio::test]
async fn get_chunk_over_network() {
    let store = Arc::new(InMemorySnapshotStore::new());
    let manifest = store
        .create_snapshot(
            5,
            sha3_256(b"state-5"),
            vec![make_entry(1, 100), make_entry(2, 200), make_entry(3, 300)],
        )
        .unwrap();
    assert!(manifest.chunk_count() >= 1);

    let (handle_a, handle_b) = start_pair_with_provider(Arc::clone(&store)).await;

    let response = handle_b
        .request_snapshot(
            handle_a.local_peer_id(),
            SnapshotRequest::GetChunk {
                height: 5,
                index: 0,
            },
        )
        .await
        .unwrap();

    match response {
        SnapshotResponse::Chunk(Some(chunk)) => {
            assert!(chunk.verify(&manifest.chunk_hashes[0]));
        }
        other => panic!("expected Chunk(Some(_)), got {other:?}"),
    }
}

#[tokio::test]
async fn missing_snapshot_returns_none() {
    let store = Arc::new(InMemorySnapshotStore::new());
    let (handle_a, handle_b) = start_pair_with_provider(Arc::clone(&store)).await;

    let response = handle_b
        .request_snapshot(
            handle_a.local_peer_id(),
            SnapshotRequest::GetManifest { height: 9999 },
        )
        .await
        .unwrap();

    assert!(matches!(response, SnapshotResponse::Manifest(None)));
}

#[tokio::test]
async fn end_to_end_snapshot_reconstruction() {
    // Server has a snapshot of 5 accounts.
    let server_store = Arc::new(InMemorySnapshotStore::new());
    let entries = vec![
        make_entry(1, 100),
        make_entry(2, 200),
        make_entry(3, 300),
        make_entry(4, 400),
        make_entry(5, 500),
    ];
    let manifest = server_store
        .create_snapshot(99, sha3_256(b"state-99"), entries.clone())
        .unwrap();

    let (handle_a, handle_b) = start_pair_with_provider(Arc::clone(&server_store)).await;

    // Client B fetches the manifest.
    let resp = handle_b
        .request_snapshot(
            handle_a.local_peer_id(),
            SnapshotRequest::GetManifest { height: 99 },
        )
        .await
        .unwrap();
    let received_manifest = match resp {
        SnapshotResponse::Manifest(Some(m)) => m,
        other => panic!("expected manifest, got {other:?}"),
    };
    assert_eq!(received_manifest.commitment(), manifest.commitment());

    // Client B fetches all chunks and reconstructs the state.
    let mut all_entries: Vec<SnapshotEntry> = Vec::new();
    for index in 0..received_manifest.chunk_count() as u32 {
        let resp = handle_b
            .request_snapshot(
                handle_a.local_peer_id(),
                SnapshotRequest::GetChunk { height: 99, index },
            )
            .await
            .unwrap();
        let chunk = match resp {
            SnapshotResponse::Chunk(Some(c)) => c,
            other => panic!("expected chunk {index}, got {other:?}"),
        };
        // Verify chunk hash matches the manifest.
        assert!(chunk.verify(&received_manifest.chunk_hashes[index as usize]));
        // Decompress and accumulate.
        let chunk_entries = karoowa_storage::decompress_chunk(&chunk.data).unwrap();
        all_entries.extend(chunk_entries);
    }

    // Verify all original entries are present.
    assert_eq!(all_entries.len(), entries.len());
    for original in &entries {
        assert!(all_entries.iter().any(
            |e| e.address == original.address && e.account.balance == original.account.balance
        ));
    }
}

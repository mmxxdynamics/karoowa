//! Integration test: two in-process Karoowa nodes exchange headers and
//! state proofs over the libp2p `/karoowa/light/1` request-response protocol.

use async_trait::async_trait;
use karoowa_core::BlockHeader;
use karoowa_crypto::{sha3_256, Address, Hash, Keypair};
use karoowa_network::{
    LightClientProvider, LightClientRequest, LightClientResponse, Network, NetworkConfig,
};
use karoowa_trie::{MerkleProof, SparseMerkleTrie};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Test provider that holds a header chain and a trie for state proofs.
struct TestProvider {
    headers: BTreeMap<u64, BlockHeader>,
    trie: Mutex<SparseMerkleTrie>,
}

impl TestProvider {
    fn new(headers: Vec<BlockHeader>, trie: SparseMerkleTrie) -> Self {
        TestProvider {
            headers: headers.into_iter().map(|h| (h.height, h)).collect(),
            trie: Mutex::new(trie),
        }
    }
}

#[async_trait]
impl LightClientProvider for TestProvider {
    async fn get_header(&self, height: u64) -> Option<BlockHeader> {
        self.headers.get(&height).cloned()
    }

    async fn get_header_range(&self, from: u64, to: u64) -> Vec<BlockHeader> {
        (from..=to)
            .filter_map(|h| self.headers.get(&h).cloned())
            .collect()
    }

    async fn get_state_proof(&self, key: &[u8], height: u64) -> Option<MerkleProof> {
        if !self.headers.contains_key(&height) {
            return None;
        }
        let trie = self.trie.lock().await;
        Some(trie.proof(key))
    }
}

fn addr(seed: u8) -> Address {
    Keypair::from_seed(&[seed; 32]).address()
}

fn make_header(height: u64, parent: Hash, state_root: Hash, proposer: Address) -> BlockHeader {
    BlockHeader {
        parent_hash: parent,
        state_root,
        tx_root: Hash::ZERO,
        receipt_root: Hash::ZERO,
        height,
        timestamp: 1700000000 + height,
        proposer,
        consensus_data: vec![],
    }
}

async fn start_pair_with_provider(
    provider: Arc<TestProvider>,
) -> (
    karoowa_network::NetworkHandle,
    karoowa_network::NetworkHandle,
) {
    let config_a = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: vec![],
        keypair_seed: Some([10; 32]),
        ..NetworkConfig::default()
    };
    let handle_a = Network::start(config_a).await.unwrap();
    handle_a.set_light_provider(provider).await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;
    let addrs_a = handle_a.listen_addresses().await.unwrap();
    assert!(!addrs_a.is_empty());

    let config_b = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: addrs_a,
        keypair_seed: Some([11; 32]),
        ..NetworkConfig::default()
    };
    let handle_b = Network::start(config_b).await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(handle_a.peer_count(), 1);
    assert_eq!(handle_b.peer_count(), 1);

    (handle_a, handle_b)
}

#[tokio::test]
async fn get_header_over_network() {
    let h0 = make_header(0, Hash::ZERO, sha3_256(b"genesis"), addr(1));
    let h1 = make_header(1, h0.hash(), sha3_256(b"state-1"), addr(2));

    let provider = Arc::new(TestProvider::new(
        vec![h0.clone(), h1.clone()],
        SparseMerkleTrie::new(),
    ));
    let (handle_a, handle_b) = start_pair_with_provider(provider).await;

    let response = handle_b
        .request_light(
            handle_a.local_peer_id(),
            LightClientRequest::GetHeader { height: 1 },
        )
        .await
        .unwrap();

    match response {
        LightClientResponse::Header(Some(received)) => {
            assert_eq!(received.hash(), h1.hash());
        }
        other => panic!("expected Header(Some(_)), got {other:?}"),
    }
}

#[tokio::test]
async fn get_header_range_over_network() {
    let h0 = make_header(0, Hash::ZERO, sha3_256(b"s0"), addr(1));
    let h1 = make_header(1, h0.hash(), sha3_256(b"s1"), addr(2));
    let h2 = make_header(2, h1.hash(), sha3_256(b"s2"), addr(3));
    let h3 = make_header(3, h2.hash(), sha3_256(b"s3"), addr(4));

    let provider = Arc::new(TestProvider::new(
        vec![h0, h1.clone(), h2.clone(), h3.clone()],
        SparseMerkleTrie::new(),
    ));
    let (handle_a, handle_b) = start_pair_with_provider(provider).await;

    let response = handle_b
        .request_light(
            handle_a.local_peer_id(),
            LightClientRequest::GetHeaderRange { from: 1, to: 3 },
        )
        .await
        .unwrap();

    match response {
        LightClientResponse::Headers(headers) => {
            assert_eq!(headers.len(), 3);
            assert_eq!(headers[0].hash(), h1.hash());
            assert_eq!(headers[2].hash(), h3.hash());
        }
        other => panic!("expected Headers, got {other:?}"),
    }
}

#[tokio::test]
async fn missing_header_returns_none() {
    let h0 = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
    let provider = Arc::new(TestProvider::new(vec![h0], SparseMerkleTrie::new()));
    let (handle_a, handle_b) = start_pair_with_provider(provider).await;

    let response = handle_b
        .request_light(
            handle_a.local_peer_id(),
            LightClientRequest::GetHeader { height: 999 },
        )
        .await
        .unwrap();

    assert!(matches!(response, LightClientResponse::Header(None)));
}

#[tokio::test]
async fn end_to_end_state_proof_over_network() {
    // Build a real trie with a known state.
    let mut trie = SparseMerkleTrie::new();
    trie.insert(b"alice", b"balance:1000".to_vec());
    trie.insert(b"bob", b"balance:2000".to_vec());
    let state_root = trie.root();

    // Build a header committing to that state root.
    let h0 = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
    let h1 = make_header(1, h0.hash(), state_root, addr(2));

    let provider = Arc::new(TestProvider::new(vec![h0, h1.clone()], trie));
    let (handle_a, handle_b) = start_pair_with_provider(provider).await;

    // Client B fetches the header and the proof, then verifies locally.
    let header_resp = handle_b
        .request_light(
            handle_a.local_peer_id(),
            LightClientRequest::GetHeader { height: 1 },
        )
        .await
        .unwrap();
    let received_header = match header_resp {
        LightClientResponse::Header(Some(h)) => h,
        other => panic!("expected header, got {other:?}"),
    };
    assert_eq!(received_header.state_root, state_root);

    let proof_resp = handle_b
        .request_light(
            handle_a.local_peer_id(),
            LightClientRequest::GetStateProof {
                key: b"alice".to_vec(),
                height: 1,
            },
        )
        .await
        .unwrap();
    let proof = match proof_resp {
        LightClientResponse::StateProof(Some(p)) => p,
        other => panic!("expected proof, got {other:?}"),
    };

    // Verify the proof against the state root from the header.
    assert!(proof.verify(&received_header.state_root).is_ok());
    assert_eq!(proof.value, Some(b"balance:1000".to_vec()));
}

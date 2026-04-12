//! Integration test: two in-process Karoowa nodes exchange bridge packets
//! over the libp2p `/karoowa/bridge/1` request-response protocol.

use async_trait::async_trait;
use karoowa_bridge::{
    Acknowledgement, BridgePacket, BridgeRelayer, EscrowStore, InMemoryEscrow, PacketProof,
};
use karoowa_crypto::{sha3_256, Address, Hash};
use karoowa_network::{
    BridgeProtocolProvider, BridgeRequest, BridgeResponse, Network, NetworkConfig,
};
use karoowa_trie::SparseMerkleTrie;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Bridge provider that wraps an in-process relayer plus its source-side
/// commitment trie. Used by the destination node to process incoming
/// `SubmitPacket` requests from the source node.
struct DestNodeProvider {
    /// Relayer with source = remote (we don't actually escrow on the source
    /// side here; the source node holds its own escrow), dest = local.
    relayer: Arc<BridgeRelayer<NoopEscrow, InMemoryEscrow>>,
    /// Cached acks for previously processed packets.
    acks: Mutex<std::collections::HashMap<Hash, Acknowledgement>>,
}

/// No-op escrow used on the destination side's view of the source escrow
/// (the destination doesn't lock or release source-chain tokens; it only
/// mints/burns wrapped tokens locally).
struct NoopEscrow;

impl EscrowStore for NoopEscrow {
    fn lock(&self, _: &Address, _: &str, _: u64) -> Result<(), karoowa_bridge::BridgeError> {
        Ok(())
    }
    fn release(&self, _: &Address, _: &str, _: u64) -> Result<(), karoowa_bridge::BridgeError> {
        Ok(())
    }
    fn mint(&self, _: &Address, _: &str, _: u64) -> Result<(), karoowa_bridge::BridgeError> {
        Ok(())
    }
    fn burn(&self, _: &Address, _: &str, _: u64) -> Result<(), karoowa_bridge::BridgeError> {
        Ok(())
    }
    fn balance_of(&self, _: &Address, _: &str) -> u64 {
        0
    }
    fn native_balance_of(&self, _: &Address, _: &str) -> u64 {
        0
    }
    fn escrowed(&self, _: &str) -> u64 {
        0
    }
}

#[async_trait]
impl BridgeProtocolProvider for DestNodeProvider {
    async fn submit_packet(
        &self,
        packet: BridgePacket,
        proof: PacketProof,
        source_state_root: Hash,
    ) -> Acknowledgement {
        match self
            .relayer
            .receive_packet(&packet, &proof, &source_state_root)
        {
            Ok(ack) => {
                self.acks.lock().await.insert(ack.packet_hash, ack.clone());
                ack
            }
            Err(e) => Acknowledgement {
                packet_hash: packet.hash(),
                success: false,
                error: Some(e.to_string()),
            },
        }
    }

    async fn get_packet_proof(&self, _packet_hash: &Hash) -> Option<PacketProof> {
        None
    }

    async fn get_acknowledgement(&self, packet_hash: &Hash) -> Option<Acknowledgement> {
        self.acks.lock().await.get(packet_hash).cloned()
    }
}

fn addr(seed: u8) -> Address {
    Address::from_public_key(&[seed; 32])
}

fn make_packet(seq: u64, sender: Address, recipient: Address, amount: u64) -> BridgePacket {
    BridgePacket {
        source_chain: "karoowa-a".into(),
        dest_chain: "karoowa-b".into(),
        sequence: seq,
        sender,
        recipient,
        amount,
        denom: "kar".into(),
        timeout_height: 9999,
    }
}

/// Build a real Merkle proof committing the packet's hash under the
/// expected commitment key.
fn build_commitment_proof(packet: &BridgePacket) -> (PacketProof, Hash) {
    let mut trie = SparseMerkleTrie::new();
    let key = packet.commitment_key();
    let value = packet.hash().as_bytes().to_vec();
    trie.insert(&key, value);
    let state_root = trie.root();
    let proof = trie.proof(&key);
    (
        PacketProof {
            packet: packet.clone(),
            source_height: 100,
            proof,
        },
        state_root,
    )
}

async fn start_pair_with_provider(
    provider: Arc<DestNodeProvider>,
) -> (
    karoowa_network::NetworkHandle,
    karoowa_network::NetworkHandle,
) {
    // Destination node A — installs the bridge provider.
    let config_a = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: vec![],
        keypair_seed: Some([20; 32]),
        ..NetworkConfig::default()
    };
    let handle_a = Network::start(config_a).await.unwrap();
    handle_a.set_bridge_provider(provider).await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;
    let addrs_a = handle_a.listen_addresses().await.unwrap();
    assert!(!addrs_a.is_empty());

    // Source node B — connects to A.
    let config_b = NetworkConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootnodes: addrs_a,
        keypair_seed: Some([21; 32]),
        ..NetworkConfig::default()
    };
    let handle_b = Network::start(config_b).await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(handle_a.peer_count(), 1);
    assert_eq!(handle_b.peer_count(), 1);

    (handle_a, handle_b)
}

#[tokio::test]
async fn submit_packet_over_network() {
    let dest_escrow = InMemoryEscrow::new();
    let relayer = Arc::new(BridgeRelayer::new(NoopEscrow, dest_escrow));

    let provider = Arc::new(DestNodeProvider {
        relayer: relayer.clone(),
        acks: Mutex::new(Default::default()),
    });

    let (handle_a, handle_b) = start_pair_with_provider(provider).await;

    let packet = make_packet(1, addr(1), addr(2), 1000);
    let (proof, state_root) = build_commitment_proof(&packet);

    let response = handle_b
        .request_bridge(
            handle_a.local_peer_id(),
            BridgeRequest::SubmitPacket {
                packet: packet.clone(),
                proof,
                source_state_root: state_root,
            },
        )
        .await
        .unwrap();

    match response {
        BridgeResponse::Acknowledgement(ack) => {
            assert!(ack.success, "ack error: {:?}", ack.error);
            assert_eq!(ack.packet_hash, packet.hash());
        }
        other => panic!("expected Acknowledgement, got {other:?}"),
    }

    // Verify the wrapped tokens were minted on the destination.
    assert_eq!(relayer.dest.balance_of(&addr(2), "ibc/kar"), 1000);
}

#[tokio::test]
async fn duplicate_packet_rejected_over_network() {
    let dest_escrow = InMemoryEscrow::new();
    let relayer = Arc::new(BridgeRelayer::new(NoopEscrow, dest_escrow));
    let provider = Arc::new(DestNodeProvider {
        relayer: relayer.clone(),
        acks: Mutex::new(Default::default()),
    });
    let (handle_a, handle_b) = start_pair_with_provider(provider).await;

    let packet = make_packet(1, addr(1), addr(2), 500);
    let (proof, state_root) = build_commitment_proof(&packet);

    // First submission succeeds.
    let response = handle_b
        .request_bridge(
            handle_a.local_peer_id(),
            BridgeRequest::SubmitPacket {
                packet: packet.clone(),
                proof: proof.clone(),
                source_state_root: state_root,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        response,
        BridgeResponse::Acknowledgement(ref a) if a.success
    ));

    // Replay attempt — provider returns a failure ack.
    let response = handle_b
        .request_bridge(
            handle_a.local_peer_id(),
            BridgeRequest::SubmitPacket {
                packet: packet.clone(),
                proof: proof.clone(),
                source_state_root: state_root,
            },
        )
        .await
        .unwrap();
    match response {
        BridgeResponse::Acknowledgement(ack) => {
            assert!(!ack.success);
            assert!(ack.error.unwrap_or_default().contains("already processed"));
        }
        other => panic!("expected failure ack, got {other:?}"),
    }

    // Wrapped balance should still be 500 (the dup didn't double-mint).
    assert_eq!(relayer.dest.balance_of(&addr(2), "ibc/kar"), 500);
}

#[tokio::test]
async fn forged_proof_rejected_over_network() {
    let dest_escrow = InMemoryEscrow::new();
    let relayer = Arc::new(BridgeRelayer::new(NoopEscrow, dest_escrow));
    let provider = Arc::new(DestNodeProvider {
        relayer: relayer.clone(),
        acks: Mutex::new(Default::default()),
    });
    let (handle_a, handle_b) = start_pair_with_provider(provider).await;

    let packet = make_packet(1, addr(1), addr(2), 1000);
    let (proof, _real_root) = build_commitment_proof(&packet);

    // Submit with a wrong source_state_root.
    let wrong_root = sha3_256(b"forged");
    let response = handle_b
        .request_bridge(
            handle_a.local_peer_id(),
            BridgeRequest::SubmitPacket {
                packet: packet.clone(),
                proof,
                source_state_root: wrong_root,
            },
        )
        .await
        .unwrap();

    match response {
        BridgeResponse::Acknowledgement(ack) => {
            assert!(!ack.success);
            assert!(ack
                .error
                .unwrap_or_default()
                .to_lowercase()
                .contains("commitment"));
        }
        other => panic!("expected failure ack, got {other:?}"),
    }

    // Nothing minted on the destination.
    assert_eq!(relayer.dest.balance_of(&addr(2), "ibc/kar"), 0);
}

#[tokio::test]
async fn get_acknowledgement_over_network() {
    let dest_escrow = InMemoryEscrow::new();
    let relayer = Arc::new(BridgeRelayer::new(NoopEscrow, dest_escrow));
    let provider = Arc::new(DestNodeProvider {
        relayer: relayer.clone(),
        acks: Mutex::new(Default::default()),
    });
    let (handle_a, handle_b) = start_pair_with_provider(provider).await;

    let packet = make_packet(1, addr(1), addr(2), 750);
    let (proof, state_root) = build_commitment_proof(&packet);
    let packet_hash = packet.hash();

    // Submit and process.
    handle_b
        .request_bridge(
            handle_a.local_peer_id(),
            BridgeRequest::SubmitPacket {
                packet,
                proof,
                source_state_root: state_root,
            },
        )
        .await
        .unwrap();

    // Now query for the cached ack.
    let response = handle_b
        .request_bridge(
            handle_a.local_peer_id(),
            BridgeRequest::GetAcknowledgement { packet_hash },
        )
        .await
        .unwrap();

    match response {
        BridgeResponse::Acknowledgement(ack) => {
            assert_eq!(ack.packet_hash, packet_hash);
            assert!(ack.success);
        }
        other => panic!("expected cached ack, got {other:?}"),
    }
}

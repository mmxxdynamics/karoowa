//! Bridge relayer — coordinates packet flow between two chains.
//!
//! The relayer holds:
//! - A handle to the source-chain escrow store
//! - A handle to the destination-chain escrow store
//! - A `LightClient` view of the destination chain (for relaying acks back)
//! - A set of processed packet hashes for replay protection
//!
//! In production this would run as an off-chain process, polling both
//! chains and submitting transactions. For the MVP, both chains live in
//! the same process, allowing end-to-end testing without networking.

use karoowa_crypto::Hash;
use std::collections::HashSet;
use std::sync::Mutex;

use crate::error::BridgeError;
use crate::escrow::EscrowStore;
use crate::packet::{Acknowledgement, BridgePacket, PacketProof};

/// Relayer that processes packets between a source and destination chain.
pub struct BridgeRelayer<S: EscrowStore, D: EscrowStore> {
    pub source: S,
    pub dest: D,
    /// Hashes of packets already processed on the destination side.
    processed: Mutex<HashSet<Hash>>,
}

impl<S: EscrowStore, D: EscrowStore> BridgeRelayer<S, D> {
    pub fn new(source: S, dest: D) -> Self {
        BridgeRelayer {
            source,
            dest,
            processed: Mutex::new(HashSet::new()),
        }
    }

    /// Initiate a transfer from source to destination.
    ///
    /// Locks tokens in the source escrow and returns the packet that needs
    /// to be relayed. In a real system, the packet's commitment would be
    /// stored in the source chain's state trie, and the relayer would
    /// fetch a Merkle proof of that commitment.
    pub fn send_packet(&self, packet: BridgePacket) -> Result<(), BridgeError> {
        // Lock the tokens on the source side.
        self.source
            .lock(&packet.sender, &packet.denom, packet.amount)?;

        tracing::debug!(
            packet_hash = %packet.hash(),
            sender = %packet.sender,
            recipient = %packet.recipient,
            amount = packet.amount,
            "bridge packet sent"
        );
        Ok(())
    }

    /// Receive a packet on the destination side.
    ///
    /// Verifies the source-chain commitment proof, then mints wrapped
    /// tokens to the recipient. Rejects duplicate packets via the
    /// processed-hashes set.
    pub fn receive_packet(
        &self,
        packet: &BridgePacket,
        commitment_proof: &PacketProof,
        source_state_root: &Hash,
    ) -> Result<Acknowledgement, BridgeError> {
        let packet_hash = packet.hash();

        // Replay protection: reject duplicates.
        {
            let processed = self.processed.lock().unwrap();
            if processed.contains(&packet_hash) {
                return Err(BridgeError::DuplicatePacket(packet_hash));
            }
        }

        // Verify the proof against the source state root.
        commitment_proof
            .proof
            .verify(source_state_root)
            .map_err(|e| BridgeError::CommitmentInvalid(e.to_string()))?;

        // Verify the proof key matches the packet's commitment key.
        let expected_key_hash = karoowa_crypto::sha3_256(&packet.commitment_key());
        if commitment_proof.proof.key != expected_key_hash {
            return Err(BridgeError::CommitmentInvalid(format!(
                "proof key mismatch: expected {expected_key_hash}, got {}",
                commitment_proof.proof.key
            )));
        }

        // Verify the proof's value matches the packet hash.
        let proof_value = commitment_proof
            .proof
            .value
            .as_ref()
            .ok_or_else(|| BridgeError::CommitmentInvalid("proof has no value".into()))?;
        if proof_value != packet_hash.as_bytes() {
            return Err(BridgeError::CommitmentInvalid(
                "proof value does not match packet hash".into(),
            ));
        }

        // Mint the wrapped tokens.
        let wrapped_denom = format!("ibc/{}", packet.denom);
        self.dest
            .mint(&packet.recipient, &wrapped_denom, packet.amount)?;

        // Mark as processed.
        self.processed.lock().unwrap().insert(packet_hash);

        tracing::debug!(
            packet_hash = %packet_hash,
            recipient = %packet.recipient,
            amount = packet.amount,
            "bridge packet received"
        );

        Ok(Acknowledgement {
            packet_hash,
            success: true,
            error: None,
        })
    }

    /// Initiate a return transfer (burn wrapped tokens, release native).
    ///
    /// On the destination chain, burn the wrapped tokens. The matching
    /// release on the source chain happens when the relayer delivers the
    /// return packet via [`Self::receive_return_packet`].
    pub fn send_return_packet(&self, packet: &BridgePacket) -> Result<(), BridgeError> {
        let wrapped_denom = format!("ibc/{}", packet.denom);
        self.dest
            .burn(&packet.sender, &wrapped_denom, packet.amount)?;
        Ok(())
    }

    /// Receive a return packet on the source side and release native tokens.
    pub fn receive_return_packet(&self, packet: &BridgePacket) -> Result<(), BridgeError> {
        self.source
            .release(&packet.recipient, &packet.denom, packet.amount)?;
        Ok(())
    }

    /// Whether a packet has already been processed on the destination side.
    pub fn was_processed(&self, hash: &Hash) -> bool {
        self.processed.lock().unwrap().contains(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escrow::InMemoryEscrow;
    use karoowa_crypto::Address;
    use karoowa_trie::SparseMerkleTrie;

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

    /// Helper: build a real Merkle proof committing to a packet hash.
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

    #[test]
    fn end_to_end_lock_and_mint() {
        let source = InMemoryEscrow::new();
        let dest = InMemoryEscrow::new();
        source.fund_native(addr(1), "kar", 10_000);

        let relayer = BridgeRelayer::new(source, dest);
        let packet = make_packet(1, addr(1), addr(2), 1000);

        // Sender initiates transfer on source side.
        relayer.send_packet(packet.clone()).unwrap();
        assert_eq!(relayer.source.native_balance_of(&addr(1), "kar"), 9000);
        assert_eq!(relayer.source.escrowed("kar"), 1000);

        // Relayer builds the commitment proof.
        let (proof, state_root) = build_commitment_proof(&packet);

        // Relayer delivers the packet to the destination.
        let ack = relayer
            .receive_packet(&packet, &proof, &state_root)
            .unwrap();
        assert!(ack.success);
        assert_eq!(relayer.dest.balance_of(&addr(2), "ibc/kar"), 1000);
    }

    #[test]
    fn end_to_end_burn_and_release() {
        let source = InMemoryEscrow::new();
        let dest = InMemoryEscrow::new();
        source.fund_native(addr(1), "kar", 5000);
        let relayer = BridgeRelayer::new(source, dest);

        // Forward leg.
        let outbound = make_packet(1, addr(1), addr(2), 2000);
        relayer.send_packet(outbound.clone()).unwrap();
        let (proof, root) = build_commitment_proof(&outbound);
        relayer.receive_packet(&outbound, &proof, &root).unwrap();
        assert_eq!(relayer.dest.balance_of(&addr(2), "ibc/kar"), 2000);

        // Return leg: addr(2) burns wrapped tokens to send back to addr(1).
        let inbound = BridgePacket {
            source_chain: "karoowa-b".into(),
            dest_chain: "karoowa-a".into(),
            sequence: 1,
            sender: addr(2),
            recipient: addr(1),
            amount: 500,
            denom: "kar".into(),
            timeout_height: 9999,
        };
        relayer.send_return_packet(&inbound).unwrap();
        assert_eq!(relayer.dest.balance_of(&addr(2), "ibc/kar"), 1500);

        // Relayer releases on the source.
        relayer.receive_return_packet(&inbound).unwrap();
        assert_eq!(relayer.source.escrowed("kar"), 1500);
        assert_eq!(relayer.source.native_balance_of(&addr(1), "kar"), 3500);
    }

    #[test]
    fn rejects_duplicate_packet() {
        let source = InMemoryEscrow::new();
        let dest = InMemoryEscrow::new();
        source.fund_native(addr(1), "kar", 10_000);
        let relayer = BridgeRelayer::new(source, dest);

        let packet = make_packet(1, addr(1), addr(2), 1000);
        relayer.send_packet(packet.clone()).unwrap();
        let (proof, root) = build_commitment_proof(&packet);

        relayer.receive_packet(&packet, &proof, &root).unwrap();
        // Replay attempt should fail.
        let result = relayer.receive_packet(&packet, &proof, &root);
        assert!(matches!(result, Err(BridgeError::DuplicatePacket(_))));
    }

    #[test]
    fn rejects_proof_with_wrong_root() {
        let source = InMemoryEscrow::new();
        let dest = InMemoryEscrow::new();
        source.fund_native(addr(1), "kar", 10_000);
        let relayer = BridgeRelayer::new(source, dest);

        let packet = make_packet(1, addr(1), addr(2), 1000);
        relayer.send_packet(packet.clone()).unwrap();
        let (proof, _real_root) = build_commitment_proof(&packet);

        let wrong_root = karoowa_crypto::sha3_256(b"wrong");
        let result = relayer.receive_packet(&packet, &proof, &wrong_root);
        assert!(matches!(result, Err(BridgeError::CommitmentInvalid(_))));
    }

    #[test]
    fn rejects_proof_with_tampered_packet() {
        let source = InMemoryEscrow::new();
        let dest = InMemoryEscrow::new();
        source.fund_native(addr(1), "kar", 10_000);
        let relayer = BridgeRelayer::new(source, dest);

        let real_packet = make_packet(1, addr(1), addr(2), 1000);
        relayer.send_packet(real_packet.clone()).unwrap();
        let (proof, root) = build_commitment_proof(&real_packet);

        // Try to claim a different packet using the real proof.
        let fake_packet = make_packet(1, addr(1), addr(2), 999_999);
        let result = relayer.receive_packet(&fake_packet, &proof, &root);
        assert!(matches!(result, Err(BridgeError::CommitmentInvalid(_))));
    }
}

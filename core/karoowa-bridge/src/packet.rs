//! Bridge packet types.
//!
//! A `BridgePacket` represents a cross-chain message — typically a token
//! transfer — moving from a source chain to a destination chain. Packets
//! are committed on the source chain in a Merkle-provable way, then
//! relayed to the destination chain along with a proof.

use karoowa_crypto::{sha3_256, Address, Hash};
use karoowa_trie::MerkleProof;
use serde::{Deserialize, Serialize};

/// A cross-chain bridge packet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgePacket {
    /// Identifier of the source chain (e.g. "karoowa-mainnet").
    pub source_chain: String,
    /// Identifier of the destination chain.
    pub dest_chain: String,
    /// Per-channel sequence number, monotonic for replay protection.
    pub sequence: u64,
    /// Sender address on the source chain.
    pub sender: Address,
    /// Recipient address on the destination chain.
    pub recipient: Address,
    /// Native amount being bridged.
    pub amount: u64,
    /// Token denomination (e.g. "kar", "ibc/HASH" for already-wrapped tokens).
    pub denom: String,
    /// Source-chain block height after which this packet is invalid.
    pub timeout_height: u64,
}

impl BridgePacket {
    /// Compute the canonical hash of this packet.
    ///
    /// Used for commitment storage on the source chain and replay protection
    /// on the destination chain.
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(self).expect("packet serialization cannot fail");
        sha3_256(&bytes)
    }

    /// Storage key for the packet commitment on the source chain.
    ///
    /// Format: `commitments/{source_chain}/{dest_chain}/{sequence}`
    pub fn commitment_key(&self) -> Vec<u8> {
        let key = format!(
            "commitments/{}/{}/{}",
            self.source_chain, self.dest_chain, self.sequence
        );
        key.into_bytes()
    }

    /// Storage key for the receipt on the destination chain.
    ///
    /// Format: `receipts/{source_chain}/{dest_chain}/{sequence}`
    pub fn receipt_key(&self) -> Vec<u8> {
        let key = format!(
            "receipts/{}/{}/{}",
            self.source_chain, self.dest_chain, self.sequence
        );
        key.into_bytes()
    }
}

/// Acknowledgement returned by the destination chain after processing a packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acknowledgement {
    /// Hash of the packet being acknowledged.
    pub packet_hash: Hash,
    /// Whether processing succeeded.
    pub success: bool,
    /// Optional error description if `success` is false.
    pub error: Option<String>,
}

/// A packet plus the cryptographic proof that it was committed on the source chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketProof {
    /// The packet itself.
    pub packet: BridgePacket,
    /// Source-chain block height at which the commitment exists.
    pub source_height: u64,
    /// Merkle proof from the source chain's state trie.
    pub proof: MerkleProof,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_packet(seq: u64) -> BridgePacket {
        BridgePacket {
            source_chain: "karoowa-a".into(),
            dest_chain: "karoowa-b".into(),
            sequence: seq,
            sender: Address::from_public_key(&[1u8; 32]),
            recipient: Address::from_public_key(&[2u8; 32]),
            amount: 1000,
            denom: "kar".into(),
            timeout_height: 9999,
        }
    }

    #[test]
    fn hash_is_deterministic() {
        let p1 = make_packet(0);
        let p2 = make_packet(0);
        assert_eq!(p1.hash(), p2.hash());
    }

    #[test]
    fn different_sequences_different_hashes() {
        let p1 = make_packet(0);
        let p2 = make_packet(1);
        assert_ne!(p1.hash(), p2.hash());
    }

    #[test]
    fn commitment_key_includes_sequence() {
        let p = make_packet(42);
        let key = p.commitment_key();
        assert_eq!(key, b"commitments/karoowa-a/karoowa-b/42");
    }

    #[test]
    fn receipt_key_distinct_from_commitment_key() {
        let p = make_packet(42);
        assert_ne!(p.commitment_key(), p.receipt_key());
    }
}

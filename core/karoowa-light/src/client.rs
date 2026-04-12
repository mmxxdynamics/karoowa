//! `LightClient` — header chain + state proof verification.
//!
//! Stores headers in a height-keyed map. Each new header is verified against
//! the previously trusted header (parent_hash linkage) and the current
//! validator set (proposer membership). State proofs are verified by
//! checking a Merkle proof against the `state_root` of the relevant header.

use karoowa_core::BlockHeader;
use karoowa_crypto::{Address, Hash};
use karoowa_trie::MerkleProof;
use std::collections::BTreeMap;
use tracing::debug;

use crate::error::LightClientError;
use crate::validator_set::ValidatorSetView;

/// A header chain light client.
///
/// Initialized from a trusted checkpoint and an active validator set.
/// All subsequent headers are verified before being added to the store.
pub struct LightClient {
    /// Headers indexed by height. The smallest entry is the trusted checkpoint.
    headers: BTreeMap<u64, BlockHeader>,
    /// Currently active validator set view.
    validators: ValidatorSetView,
}

impl LightClient {
    /// Create a new light client from a trusted checkpoint header.
    ///
    /// Returns an error if the validator set is empty.
    pub fn new(
        checkpoint: BlockHeader,
        validators: ValidatorSetView,
    ) -> Result<Self, LightClientError> {
        if validators.is_empty() {
            return Err(LightClientError::EmptyValidatorSet);
        }
        let mut headers = BTreeMap::new();
        headers.insert(checkpoint.height, checkpoint);
        Ok(LightClient {
            headers,
            validators,
        })
    }

    /// The current head (highest synced header).
    pub fn head(&self) -> &BlockHeader {
        self.headers
            .values()
            .next_back()
            .expect("light client always has at least the checkpoint")
    }

    /// Get a header at a specific height.
    pub fn header_at(&self, height: u64) -> Option<&BlockHeader> {
        self.headers.get(&height)
    }

    /// Number of headers stored.
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    /// Whether the store is empty (always false — checkpoint is always present).
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    /// Append a new header to the chain.
    ///
    /// Verifies:
    /// 1. The proposer is in the active validator set.
    /// 2. The height is exactly head.height + 1.
    /// 3. The parent_hash matches the current head's hash.
    pub fn append_header(&mut self, header: BlockHeader) -> Result<(), LightClientError> {
        let head = self.head();
        let expected_height = head.height + 1;

        if header.height != expected_height {
            return Err(LightClientError::HeightMismatch {
                expected: expected_height,
                got: header.height,
            });
        }

        let expected_parent = head.hash();
        if header.parent_hash != expected_parent {
            return Err(LightClientError::ParentHashMismatch {
                height: header.height,
                expected: expected_parent,
                got: header.parent_hash,
            });
        }

        if !self.validators.contains(&header.proposer) {
            return Err(LightClientError::UnknownProposer {
                height: header.height,
            });
        }

        debug!(
            height = header.height,
            hash = %header.hash(),
            "light client appended header"
        );
        self.headers.insert(header.height, header);
        Ok(())
    }

    /// Update the active validator set (e.g., after a validator rotation).
    ///
    /// The new view becomes effective for header verification immediately.
    pub fn update_validator_set(&mut self, view: ValidatorSetView) -> Result<(), LightClientError> {
        if view.is_empty() {
            return Err(LightClientError::EmptyValidatorSet);
        }
        self.validators = view;
        Ok(())
    }

    /// The active validator set view.
    pub fn validator_set(&self) -> &ValidatorSetView {
        &self.validators
    }

    /// Verify a Merkle proof for an account or storage value at a given height.
    ///
    /// The proof must be verified against the `state_root` of the header
    /// at that height, which we trust because we verified the header chain
    /// from the checkpoint.
    pub fn verify_state_proof(
        &self,
        height: u64,
        proof: &MerkleProof,
    ) -> Result<(), LightClientError> {
        let header = self
            .header_at(height)
            .ok_or(LightClientError::HeaderNotFound(height))?;
        proof
            .verify(&header.state_root)
            .map_err(|e| LightClientError::ProofInvalid(e.to_string()))
    }

    /// Convenience: verify a proof and return the value if it's an inclusion proof.
    pub fn verify_and_get(
        &self,
        height: u64,
        proof: &MerkleProof,
    ) -> Result<Option<Vec<u8>>, LightClientError> {
        self.verify_state_proof(height, proof)?;
        Ok(proof.value.clone())
    }

    /// Whether an address is currently a validator (per the cached set).
    pub fn is_validator(&self, address: &Address) -> bool {
        self.validators.contains(address)
    }

    /// The trusted checkpoint header (lowest-height entry).
    pub fn checkpoint(&self) -> &BlockHeader {
        self.headers
            .values()
            .next()
            .expect("light client always has at least the checkpoint")
    }

    /// Sentinel: returns the genesis state root if the light client only
    /// has the checkpoint. Used in some bootstrap flows.
    #[allow(dead_code)]
    fn checkpoint_state_root(&self) -> Hash {
        self.checkpoint().state_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_crypto::{sha3_256, Keypair};
    use karoowa_trie::SparseMerkleTrie;

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

    fn test_validators() -> ValidatorSetView {
        ValidatorSetView::new(vec![addr(1), addr(2), addr(3), addr(4)], 0)
    }

    #[test]
    fn checkpoint_only() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let client = LightClient::new(checkpoint.clone(), test_validators()).unwrap();

        assert_eq!(client.len(), 1);
        assert_eq!(client.head().height, 0);
        assert_eq!(client.header_at(0).unwrap().hash(), checkpoint.hash());
        assert!(client.header_at(1).is_none());
    }

    #[test]
    fn empty_validator_set_rejected() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let empty = ValidatorSetView::new(vec![], 0);
        let result = LightClient::new(checkpoint, empty);
        assert!(matches!(result, Err(LightClientError::EmptyValidatorSet)));
    }

    #[test]
    fn append_valid_header() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint.clone(), test_validators()).unwrap();

        let next = make_header(1, checkpoint.hash(), sha3_256(b"state-1"), addr(2));
        client.append_header(next.clone()).unwrap();

        assert_eq!(client.len(), 2);
        assert_eq!(client.head().height, 1);
        assert_eq!(client.header_at(1).unwrap().hash(), next.hash());
    }

    #[test]
    fn reject_wrong_parent_hash() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint, test_validators()).unwrap();

        let bad = make_header(1, sha3_256(b"wrong"), Hash::ZERO, addr(2));
        let result = client.append_header(bad);
        assert!(matches!(
            result,
            Err(LightClientError::ParentHashMismatch { .. })
        ));
    }

    #[test]
    fn reject_wrong_height() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint.clone(), test_validators()).unwrap();

        // Skip from 0 directly to 5 — wrong.
        let skipped = make_header(5, checkpoint.hash(), Hash::ZERO, addr(2));
        let result = client.append_header(skipped);
        assert!(matches!(
            result,
            Err(LightClientError::HeightMismatch { .. })
        ));
    }

    #[test]
    fn reject_unknown_proposer() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint.clone(), test_validators()).unwrap();

        let intruder = addr(99); // not in the validator set
        let bad = make_header(1, checkpoint.hash(), Hash::ZERO, intruder);
        let result = client.append_header(bad);
        assert!(matches!(
            result,
            Err(LightClientError::UnknownProposer { .. })
        ));
    }

    #[test]
    fn append_chain_of_headers() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint.clone(), test_validators()).unwrap();

        let mut parent = checkpoint;
        for i in 1..=10u64 {
            let proposer = addr((i as u8 % 4) + 1);
            let next = make_header(i, parent.hash(), sha3_256(&i.to_be_bytes()), proposer);
            client.append_header(next.clone()).unwrap();
            parent = next;
        }

        assert_eq!(client.len(), 11); // checkpoint + 10
        assert_eq!(client.head().height, 10);
    }

    #[test]
    fn verify_state_proof_against_header() {
        // Build a trie with a known state.
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"alice", b"balance:1000".to_vec());
        trie.insert(b"bob", b"balance:2000".to_vec());
        let state_root = trie.root();

        // Build a header committing to that state root.
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint.clone(), test_validators()).unwrap();

        let h1 = make_header(1, checkpoint.hash(), state_root, addr(2));
        client.append_header(h1).unwrap();

        // Get a Merkle proof for "alice" and verify it through the light client.
        let proof = trie.proof(b"alice");
        let value = client.verify_and_get(1, &proof).unwrap();
        assert_eq!(value, Some(b"balance:1000".to_vec()));
    }

    #[test]
    fn reject_forged_state_proof() {
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"alice", b"balance:1000".to_vec());
        let state_root = trie.root();

        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint.clone(), test_validators()).unwrap();

        // Header committing to the REAL state root.
        let h1 = make_header(1, checkpoint.hash(), state_root, addr(2));
        client.append_header(h1).unwrap();

        // Build a DIFFERENT trie and try to forge a proof.
        let mut fake_trie = SparseMerkleTrie::new();
        fake_trie.insert(b"alice", b"balance:9999999".to_vec());
        let fake_proof = fake_trie.proof(b"alice");

        // The fake proof should fail against the real header's state root.
        let result = client.verify_state_proof(1, &fake_proof);
        assert!(matches!(result, Err(LightClientError::ProofInvalid(_))));
    }

    #[test]
    fn verify_proof_at_unknown_height_fails() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let client = LightClient::new(checkpoint, test_validators()).unwrap();

        let trie = SparseMerkleTrie::new();
        let proof = trie.proof(b"any_key");
        let result = client.verify_state_proof(999, &proof);
        assert!(matches!(result, Err(LightClientError::HeaderNotFound(999))));
    }

    #[test]
    fn update_validator_set() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint, test_validators()).unwrap();

        // Replace with a new set.
        let new_set = ValidatorSetView::new(vec![addr(5), addr(6)], 100);
        client.update_validator_set(new_set).unwrap();

        assert!(client.is_validator(&addr(5)));
        assert!(!client.is_validator(&addr(1)));
    }

    #[test]
    fn reject_empty_validator_update() {
        let checkpoint = make_header(0, Hash::ZERO, Hash::ZERO, addr(1));
        let mut client = LightClient::new(checkpoint, test_validators()).unwrap();

        let empty = ValidatorSetView::new(vec![], 0);
        let result = client.update_validator_set(empty);
        assert!(matches!(result, Err(LightClientError::EmptyValidatorSet)));
    }
}

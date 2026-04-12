//! Merkle proof generation and verification.

use karoowa_crypto::{sha3_256, Hash};
use serde::{Deserialize, Serialize};

use crate::trie::{bit_at, DEPTH};

/// A Merkle proof for a key in the Sparse Merkle Trie.
///
/// Contains the sibling hashes along the path from root to leaf.
/// Can prove both inclusion (value is Some) and exclusion (value is None).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// The key hash this proof is for.
    pub key: Hash,
    /// The value at this key (None = exclusion proof).
    pub value: Option<Vec<u8>>,
    /// Sibling hashes from root (index 0) down to leaf (index DEPTH-1).
    pub siblings: Vec<Hash>,
}

/// Errors from proof verification.
#[derive(Debug, thiserror::Error)]
pub enum ProofVerifyError {
    #[error("proof has wrong number of siblings: expected {DEPTH}, got {0}")]
    WrongSiblingCount(usize),
    #[error("computed root {computed} does not match expected root {expected}")]
    RootMismatch { computed: Hash, expected: Hash },
}

impl MerkleProof {
    /// Verify this proof against an expected root hash.
    pub fn verify(&self, expected_root: &Hash) -> Result<(), ProofVerifyError> {
        if self.siblings.len() != DEPTH {
            return Err(ProofVerifyError::WrongSiblingCount(self.siblings.len()));
        }

        // Compute the leaf hash.
        let mut current = match &self.value {
            Some(value) => {
                let mut input = Vec::with_capacity(32 + value.len());
                input.extend_from_slice(self.key.as_bytes());
                input.extend_from_slice(value);
                sha3_256(&input)
            }
            None => Hash::ZERO, // Default leaf for exclusion proofs.
        };

        // Walk up from leaf to root. Siblings are stored root-to-leaf,
        // so we iterate in reverse. At depth i (from root, 0..DEPTH),
        // the bit is bit_at(key, i).
        for i in (0..DEPTH).rev() {
            let sibling = &self.siblings[i];
            let bit = bit_at(&self.key, i);

            let mut combined = Vec::with_capacity(64);
            if bit == 0 {
                // Current is on the left, sibling on the right.
                combined.extend_from_slice(current.as_bytes());
                combined.extend_from_slice(sibling.as_bytes());
            } else {
                // Current is on the right, sibling on the left.
                combined.extend_from_slice(sibling.as_bytes());
                combined.extend_from_slice(current.as_bytes());
            }
            current = sha3_256(&combined);
        }

        if current == *expected_root {
            Ok(())
        } else {
            Err(ProofVerifyError::RootMismatch {
                computed: current,
                expected: *expected_root,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SparseMerkleTrie;

    #[test]
    fn inclusion_proof_verifies() {
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"account_1", b"balance:1000".to_vec());
        trie.insert(b"account_2", b"balance:2000".to_vec());

        let root = trie.root();
        let proof = trie.proof(b"account_1");

        assert!(proof.value.is_some());
        assert_eq!(proof.value.as_ref().unwrap(), b"balance:1000");
        assert!(proof.verify(&root).is_ok());
    }

    #[test]
    fn exclusion_proof_verifies() {
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"exists", b"yes".to_vec());

        let root = trie.root();
        let proof = trie.proof(b"does_not_exist");

        assert!(proof.value.is_none());
        assert!(proof.verify(&root).is_ok());
    }

    #[test]
    fn proof_fails_against_wrong_root() {
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"key", b"value".to_vec());

        let root = trie.root();
        let proof = trie.proof(b"key");

        let wrong_root = sha3_256(b"wrong");
        assert!(proof.verify(&wrong_root).is_err());
        assert!(proof.verify(&root).is_ok());
    }

    #[test]
    fn proof_after_update() {
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"key", b"v1".to_vec());
        let root1 = trie.root();
        let proof1 = trie.proof(b"key");
        assert!(proof1.verify(&root1).is_ok());

        trie.insert(b"key", b"v2".to_vec());
        let root2 = trie.root();
        let proof2 = trie.proof(b"key");

        assert!(proof1.verify(&root2).is_err());
        assert!(proof2.verify(&root2).is_ok());
        assert!(proof2.verify(&root1).is_err());
    }

    #[test]
    fn many_proofs() {
        let mut trie = SparseMerkleTrie::new();
        for i in 0..5u32 {
            trie.insert(&i.to_be_bytes(), i.to_be_bytes().to_vec());
        }

        let root = trie.root();

        for i in 0..5u32 {
            let proof = trie.proof(&i.to_be_bytes());
            assert!(proof.value.is_some());
            assert!(proof.verify(&root).is_ok(), "proof failed for key {i}");
        }

        // Exclusion proofs.
        for i in 5..8u32 {
            let proof = trie.proof(&i.to_be_bytes());
            assert!(proof.value.is_none());
            assert!(
                proof.verify(&root).is_ok(),
                "exclusion proof failed for key {i}"
            );
        }
    }
}

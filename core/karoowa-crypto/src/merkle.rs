//! Binary Merkle tree with SHA3-256 internal nodes.
//!
//! Used for transaction roots, receipt roots, and state roots. The tree is
//! built from a list of leaf hashes and produces a single root hash. Proofs
//! can be generated and verified for individual leaves.
//!
//! Empty trees have a root of [`Hash::ZERO`]. A single-leaf tree has a root
//! equal to the leaf hash.

use crate::hash::{sha3_256, Hash};

/// A binary Merkle tree built from leaf hashes.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// All nodes in the tree, stored level-by-level from leaves to root.
    /// The last element is the root.
    nodes: Vec<Hash>,
    /// Number of leaves.
    leaf_count: usize,
}

impl MerkleTree {
    /// Build a Merkle tree from a list of leaf hashes.
    ///
    /// If the number of leaves is odd at any level, the last node is
    /// duplicated to make it even (standard Bitcoin/Ethereum-style padding).
    pub fn from_leaves(leaves: &[Hash]) -> Self {
        if leaves.is_empty() {
            return MerkleTree {
                nodes: vec![Hash::ZERO],
                leaf_count: 0,
            };
        }

        if leaves.len() == 1 {
            return MerkleTree {
                nodes: vec![leaves[0]],
                leaf_count: 1,
            };
        }

        let mut current_level: Vec<Hash> = leaves.to_vec();
        let mut all_nodes: Vec<Hash> = current_level.clone();

        while current_level.len() > 1 {
            // Pad odd levels by duplicating the last element.
            if current_level.len() % 2 != 0 {
                current_level.push(*current_level.last().unwrap());
            }

            let mut next_level = Vec::with_capacity(current_level.len() / 2);
            for pair in current_level.chunks(2) {
                next_level.push(hash_pair(&pair[0], &pair[1]));
            }

            all_nodes.extend_from_slice(&next_level);
            current_level = next_level;
        }

        MerkleTree {
            nodes: all_nodes,
            leaf_count: leaves.len(),
        }
    }

    /// The root hash of the tree.
    pub fn root(&self) -> Hash {
        *self.nodes.last().unwrap_or(&Hash::ZERO)
    }

    /// Number of leaves in the tree.
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Generate a Merkle proof for the leaf at `index`.
    ///
    /// The proof is a list of sibling hashes from the leaf up to (but not
    /// including) the root. Returns `None` if the index is out of bounds.
    pub fn proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaf_count {
            return None;
        }

        let mut siblings = Vec::new();
        let mut current_idx = index;

        // Rebuild the tree level by level and collect siblings.
        let mut current_level: Vec<Hash> = self.nodes[..self.leaf_count].to_vec();

        while current_level.len() > 1 {
            if current_level.len() % 2 != 0 {
                current_level.push(*current_level.last().unwrap());
            }

            let sibling_idx = if current_idx % 2 == 0 {
                current_idx + 1
            } else {
                current_idx - 1
            };

            siblings.push(ProofEntry {
                hash: current_level[sibling_idx],
                is_left: current_idx % 2 != 0,
            });

            // Move up a level.
            let mut next_level = Vec::with_capacity(current_level.len() / 2);
            for pair in current_level.chunks(2) {
                next_level.push(hash_pair(&pair[0], &pair[1]));
            }
            current_idx /= 2;
            current_level = next_level;
        }

        Some(MerkleProof {
            leaf_index: index,
            siblings,
        })
    }
}

/// A Merkle proof for a single leaf.
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// The index of the leaf this proof is for.
    pub leaf_index: usize,
    /// Sibling hashes from leaf to root.
    pub siblings: Vec<ProofEntry>,
}

/// A single entry in a Merkle proof.
#[derive(Debug, Clone)]
pub struct ProofEntry {
    /// The sibling hash at this level.
    pub hash: Hash,
    /// Whether this sibling is on the left side (i.e. the proven node is right).
    pub is_left: bool,
}

/// Verify a Merkle proof against a known root and leaf.
///
/// Returns `true` if the proof is valid.
pub fn verify_proof(root: &Hash, leaf: &Hash, proof: &MerkleProof) -> bool {
    let mut current = *leaf;

    for entry in &proof.siblings {
        if entry.is_left {
            current = hash_pair(&entry.hash, &current);
        } else {
            current = hash_pair(&current, &entry.hash);
        }
    }

    current == *root
}

/// Hash two child nodes into a parent node.
fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let mut combined = Vec::with_capacity(64);
    combined.extend_from_slice(left.as_bytes());
    combined.extend_from_slice(right.as_bytes());
    sha3_256(&combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(n: u8) -> Hash {
        sha3_256(&[n])
    }

    #[test]
    fn empty_tree_has_zero_root() {
        let tree = MerkleTree::from_leaves(&[]);
        assert_eq!(tree.root(), Hash::ZERO);
        assert_eq!(tree.leaf_count(), 0);
    }

    #[test]
    fn single_leaf_tree() {
        let l = leaf(1);
        let tree = MerkleTree::from_leaves(&[l]);
        assert_eq!(tree.root(), l);
        assert_eq!(tree.leaf_count(), 1);
    }

    #[test]
    fn two_leaf_tree() {
        let l0 = leaf(0);
        let l1 = leaf(1);
        let tree = MerkleTree::from_leaves(&[l0, l1]);
        let expected_root = hash_pair(&l0, &l1);
        assert_eq!(tree.root(), expected_root);
    }

    #[test]
    fn three_leaf_tree_pads() {
        let leaves: Vec<Hash> = (0..3).map(leaf).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        assert_eq!(tree.leaf_count(), 3);
        // With padding: [l0, l1, l2, l2] → [h01, h22] → root
        let h01 = hash_pair(&leaves[0], &leaves[1]);
        let h22 = hash_pair(&leaves[2], &leaves[2]);
        let expected_root = hash_pair(&h01, &h22);
        assert_eq!(tree.root(), expected_root);
    }

    #[test]
    fn four_leaf_tree() {
        let leaves: Vec<Hash> = (0..4).map(leaf).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let h01 = hash_pair(&leaves[0], &leaves[1]);
        let h23 = hash_pair(&leaves[2], &leaves[3]);
        let expected_root = hash_pair(&h01, &h23);
        assert_eq!(tree.root(), expected_root);
    }

    #[test]
    fn proof_verifies_all_leaves() {
        let leaves: Vec<Hash> = (0..7).map(leaf).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();

        for (i, leaf_hash) in leaves.iter().enumerate() {
            let proof = tree.proof(i).expect("proof should exist");
            assert!(
                verify_proof(&root, leaf_hash, &proof),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn proof_fails_for_wrong_leaf() {
        let leaves: Vec<Hash> = (0..4).map(leaf).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();
        let proof = tree.proof(0).unwrap();
        let wrong_leaf = leaf(99);
        assert!(!verify_proof(&root, &wrong_leaf, &proof));
    }

    #[test]
    fn proof_out_of_bounds_returns_none() {
        let leaves: Vec<Hash> = (0..4).map(leaf).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        assert!(tree.proof(4).is_none());
        assert!(tree.proof(100).is_none());
    }

    #[test]
    fn deterministic_root() {
        let leaves: Vec<Hash> = (0..10).map(leaf).collect();
        let tree1 = MerkleTree::from_leaves(&leaves);
        let tree2 = MerkleTree::from_leaves(&leaves);
        assert_eq!(tree1.root(), tree2.root());
    }
}

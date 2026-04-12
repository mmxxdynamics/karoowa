//! Sparse Merkle Trie implementation.
//!
//! Uses a 256-bit key space (SHA3-256 of the user key). Empty subtrees
//! are represented by precomputed default hashes. The tree is computed
//! recursively but short-circuits empty subtrees, giving O(N * log N)
//! instead of exponential complexity.

use karoowa_crypto::{sha3_256, Hash};
use std::collections::BTreeMap;

use crate::proof::MerkleProof;

/// The depth of the Sparse Merkle Trie (256 bits = SHA3-256 output).
pub(crate) const DEPTH: usize = 256;

/// Precomputed default hashes for each level of the trie.
fn default_hashes() -> Vec<Hash> {
    let mut defaults = vec![Hash::ZERO; DEPTH + 1];
    for i in 1..=DEPTH {
        let mut combined = Vec::with_capacity(64);
        combined.extend_from_slice(defaults[i - 1].as_bytes());
        combined.extend_from_slice(defaults[i - 1].as_bytes());
        defaults[i] = sha3_256(&combined);
    }
    defaults
}

/// A Sparse Merkle Trie with 256-bit keys.
pub struct SparseMerkleTrie {
    /// Leaf values indexed by their key hash (sorted for deterministic iteration).
    leaves: BTreeMap<Hash, Vec<u8>>,
    /// Precomputed default hashes per level.
    defaults: Vec<Hash>,
}

impl SparseMerkleTrie {
    /// Create an empty trie.
    pub fn new() -> Self {
        SparseMerkleTrie {
            leaves: BTreeMap::new(),
            defaults: default_hashes(),
        }
    }

    /// Insert or update a key-value pair.
    pub fn insert(&mut self, key: &[u8], value: Vec<u8>) {
        let key_hash = sha3_256(key);
        self.leaves.insert(key_hash, value);
    }

    /// Get a value by key.
    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        let key_hash = sha3_256(key);
        self.leaves.get(&key_hash)
    }

    /// Delete a key.
    pub fn delete(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let key_hash = sha3_256(key);
        self.leaves.remove(&key_hash)
    }

    /// Returns the number of entries in the trie.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Returns true if the trie is empty.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Compute the root hash.
    pub fn root(&self) -> Hash {
        if self.leaves.is_empty() {
            return self.defaults[DEPTH];
        }

        // Collect leaves as (key_hash, leaf_hash) pairs, sorted by key_hash.
        // BTreeMap iteration is already sorted.
        let leaves: Vec<(Hash, Hash)> = self
            .leaves
            .iter()
            .map(|(k, v)| (*k, leaf_hash(k, v)))
            .collect();

        self.compute_subtree_hash(&leaves, 0)
    }

    /// Recursively compute the hash of a subtree.
    ///
    /// `leaves` is a slice of (key_hash, leaf_hash) pairs that fall under
    /// the current subtree, sorted by key_hash. `depth` is the current
    /// bit-depth from the root (0 = root, DEPTH = leaves).
    ///
    /// Short-circuits empty subtrees to the default hash.
    fn compute_subtree_hash(&self, leaves: &[(Hash, Hash)], depth: usize) -> Hash {
        // Empty subtree → default hash for this level.
        if leaves.is_empty() {
            return self.defaults[DEPTH - depth];
        }

        // Leaf level → return the single leaf's hash.
        if depth == DEPTH {
            debug_assert_eq!(leaves.len(), 1);
            return leaves[0].1;
        }

        // Single-leaf subtree at non-leaf level → walk down, inserting
        // default hashes for the empty sibling at each level.
        if leaves.len() == 1 {
            let bit = bit_at(&leaves[0].0, depth);
            let child = self.compute_subtree_hash(leaves, depth + 1);
            let default = self.defaults[DEPTH - depth - 1];
            return if bit == 0 {
                hash_pair(&child, &default)
            } else {
                hash_pair(&default, &child)
            };
        }

        // Split leaves into left (bit=0) and right (bit=1) based on the
        // current depth's bit. Since leaves are sorted by key_hash, and
        // the bit at `depth` determines the sort partition, we can binary
        // search or linear scan to find the split point.
        let split = leaves
            .iter()
            .position(|(k, _)| bit_at(k, depth) == 1)
            .unwrap_or(leaves.len());
        let (left, right) = leaves.split_at(split);

        let left_hash = self.compute_subtree_hash(left, depth + 1);
        let right_hash = self.compute_subtree_hash(right, depth + 1);
        hash_pair(&left_hash, &right_hash)
    }

    /// Generate a Merkle proof for a key.
    ///
    /// Returns sibling hashes from root (index 0) down to leaf (index 255).
    /// Note: this is "top-down" ordering — verification walks in reverse.
    pub fn proof(&self, key: &[u8]) -> MerkleProof {
        let key_hash = sha3_256(key);
        let value = self.leaves.get(&key_hash).cloned();

        let leaves: Vec<(Hash, Hash)> = self
            .leaves
            .iter()
            .map(|(k, v)| (*k, leaf_hash(k, v)))
            .collect();

        let mut siblings = Vec::with_capacity(DEPTH);
        self.collect_siblings(&leaves, 0, &key_hash, &mut siblings);

        MerkleProof {
            key: key_hash,
            value,
            siblings,
        }
    }

    /// Walk the trie along the key's path, recording sibling hashes.
    /// Siblings are recorded in order from root to leaf.
    fn collect_siblings(
        &self,
        leaves: &[(Hash, Hash)],
        depth: usize,
        target_key: &Hash,
        siblings: &mut Vec<Hash>,
    ) {
        if depth == DEPTH {
            return;
        }

        let target_bit = bit_at(target_key, depth);
        let split = leaves
            .iter()
            .position(|(k, _)| bit_at(k, depth) == 1)
            .unwrap_or(leaves.len());
        let (left, right) = leaves.split_at(split);

        let (own_subtree, sibling_subtree) = if target_bit == 0 {
            (left, right)
        } else {
            (right, left)
        };

        let sibling_hash = self.compute_subtree_hash(sibling_subtree, depth + 1);
        siblings.push(sibling_hash);

        self.collect_siblings(own_subtree, depth + 1, target_key, siblings);
    }
}

impl Default for SparseMerkleTrie {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a leaf: `SHA3(key_hash || value)`.
fn leaf_hash(key_hash: &Hash, value: &[u8]) -> Hash {
    let mut input = Vec::with_capacity(32 + value.len());
    input.extend_from_slice(key_hash.as_bytes());
    input.extend_from_slice(value);
    sha3_256(&input)
}

/// Hash two sibling nodes together.
fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let mut combined = Vec::with_capacity(64);
    combined.extend_from_slice(left.as_bytes());
    combined.extend_from_slice(right.as_bytes());
    sha3_256(&combined)
}

/// Return the bit at the given depth of a Hash (0-indexed, MSB first).
pub(crate) fn bit_at(hash: &Hash, depth: usize) -> u8 {
    let byte = hash.as_bytes()[depth / 8];
    (byte >> (7 - (depth % 8))) & 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_trie_has_default_root() {
        let trie = SparseMerkleTrie::new();
        let root = trie.root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn insert_changes_root() {
        let empty = SparseMerkleTrie::new();
        let root_before = empty.root();
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"key1", b"value1".to_vec());
        let root_after = trie.root();
        assert_ne!(root_before, root_after);
    }

    #[test]
    fn same_data_same_root() {
        let mut trie1 = SparseMerkleTrie::new();
        let mut trie2 = SparseMerkleTrie::new();

        trie1.insert(b"a", b"1".to_vec());
        trie1.insert(b"b", b"2".to_vec());

        trie2.insert(b"b", b"2".to_vec());
        trie2.insert(b"a", b"1".to_vec());

        assert_eq!(trie1.root(), trie2.root());
    }

    #[test]
    fn different_data_different_root() {
        let mut trie1 = SparseMerkleTrie::new();
        let mut trie2 = SparseMerkleTrie::new();

        trie1.insert(b"key", b"value1".to_vec());
        trie2.insert(b"key", b"value2".to_vec());

        assert_ne!(trie1.root(), trie2.root());
    }

    #[test]
    fn get_returns_inserted_value() {
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"hello", b"world".to_vec());
        assert_eq!(trie.get(b"hello"), Some(&b"world".to_vec()));
        assert_eq!(trie.get(b"missing"), None);
    }

    #[test]
    fn delete_removes_value_and_changes_root() {
        let mut trie = SparseMerkleTrie::new();
        let empty_root = trie.root();

        trie.insert(b"key", b"value".to_vec());
        assert_ne!(trie.root(), empty_root);

        trie.delete(b"key");
        assert_eq!(trie.get(b"key"), None);
        assert_eq!(trie.root(), empty_root);
    }

    #[test]
    fn update_value_changes_root() {
        let mut trie = SparseMerkleTrie::new();
        trie.insert(b"key", b"v1".to_vec());
        let root1 = trie.root();

        trie.insert(b"key", b"v2".to_vec());
        let root2 = trie.root();

        assert_ne!(root1, root2);
        assert_eq!(trie.get(b"key"), Some(&b"v2".to_vec()));
    }

    #[test]
    fn many_inserts() {
        let mut trie = SparseMerkleTrie::new();
        for i in 0..10u32 {
            trie.insert(&i.to_be_bytes(), i.to_be_bytes().to_vec());
        }
        assert_eq!(trie.len(), 10);

        let root = trie.root();
        assert_ne!(root, Hash::ZERO);

        for i in 0..10u32 {
            assert_eq!(trie.get(&i.to_be_bytes()), Some(&i.to_be_bytes().to_vec()));
        }
    }
}

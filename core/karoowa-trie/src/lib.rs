//! Karoowa Sparse Merkle Trie.
//!
//! A 256-bit key-space Sparse Merkle Trie (SMT) for cryptographic state
//! commitment. Every account and storage slot is represented as a leaf in
//! the trie, and the trie root is a deterministic commitment to the entire
//! state.
//!
//! Features:
//! - **O(log n) proofs** for inclusion and exclusion
//! - **Deterministic root** — same key-value pairs always produce the same root
//! - **Efficient updates** — only affected branch hashes are recomputed
//!
//! The trie uses SHA3-256 as its hash function, consistent with the rest
//! of Karoowa's cryptographic stack.

pub mod proof;
pub mod trie;

pub use proof::{MerkleProof, ProofVerifyError};
pub use trie::SparseMerkleTrie;

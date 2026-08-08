//! Karoowa crypto primitives.
//!
//! This crate provides the foundational cryptographic types used across the
//! Karoowa workspace:
//!
//! - [`struct@Hash`] — 32-byte hash value, plus [`sha3_256`] and [`blake3_hash`] functions.
//! - [`Address`] — 20-byte account address derived from a public key.
//! - [`Keypair`] — Ed25519 signing keypair with OS-entropy generation.
//! - [`Signature`] — Ed25519 signature with signer verification.
//! - [`MerkleTree`] — Binary Merkle tree with SHA3-256 internal nodes.
//! - [`write_secret_file`] — write key material with owner-only permissions.
//!
//! All cryptographic operations use audited, well-tested crates (`sha3`,
//! `blake3`, `ed25519-dalek`). No hand-rolled crypto.

pub mod address;
pub mod hash;
pub mod keypair;
pub mod merkle;
pub mod secret_file;

pub use address::{Address, AddressError};
pub use hash::{blake3_hash, sha3_256, Hash, HashError};
pub use keypair::{Keypair, Signature, SignatureError};
pub use merkle::{verify_proof, MerkleProof, MerkleTree};
pub use secret_file::write_secret_file;

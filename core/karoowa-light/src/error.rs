//! Light client error types.

use karoowa_crypto::Hash;

/// Errors produced by the light client.
#[derive(Debug, thiserror::Error)]
pub enum LightClientError {
    /// The header's parent hash doesn't match the previous trusted header.
    #[error("parent_hash mismatch at height {height}: expected {expected}, got {got}")]
    ParentHashMismatch {
        height: u64,
        expected: Hash,
        got: Hash,
    },

    /// The header's height is wrong (must be parent height + 1).
    #[error("height mismatch: expected {expected}, got {got}")]
    HeightMismatch { expected: u64, got: u64 },

    /// The header's proposer is not in the active validator set.
    #[error("proposer is not a known validator at height {height}")]
    UnknownProposer { height: u64 },

    /// The header is not validly signed by its claimed proposer.
    #[error("invalid proposer signature at height {height}")]
    InvalidProposerSignature { height: u64 },

    /// Tried to query a height that hasn't been synced yet.
    #[error("header at height {0} not in light client store")]
    HeaderNotFound(u64),

    /// Merkle proof verification failed.
    #[error("Merkle proof verification failed: {0}")]
    ProofInvalid(String),

    /// The validator set is empty (cannot start a light client without one).
    #[error("validator set is empty")]
    EmptyValidatorSet,
}

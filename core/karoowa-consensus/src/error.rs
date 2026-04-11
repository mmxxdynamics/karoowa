//! Consensus error types.

/// Errors produced by consensus operations.
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    /// The block failed validation (wrong proposer, bad signature, etc.).
    #[error("invalid block: {0}")]
    InvalidBlock(String),

    /// This node is not the current leader and cannot propose.
    #[error("not the current leader")]
    NotLeader,

    /// The validator set is empty or misconfigured.
    #[error("invalid validator set: {0}")]
    InvalidValidatorSet(String),

    /// Storage layer error during consensus operations.
    #[error("storage error: {0}")]
    Storage(String),

    /// The parent block was not found in storage.
    #[error("parent block not found: {0}")]
    ParentNotFound(String),
}

impl From<karoowa_storage::StorageError> for ConsensusError {
    fn from(e: karoowa_storage::StorageError) -> Self {
        ConsensusError::Storage(e.to_string())
    }
}

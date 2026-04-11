//! Storage error types.

/// Errors produced by storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The underlying database returned an error.
    #[error("database error: {0}")]
    Database(String),

    /// Serialization / deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// A consistency invariant was violated (e.g. duplicate block height).
    #[error("consistency error: {0}")]
    Consistency(String),
}

impl From<rocksdb::Error> for StorageError {
    fn from(e: rocksdb::Error) -> Self {
        StorageError::Database(e.to_string())
    }
}

impl From<bincode::Error> for StorageError {
    fn from(e: bincode::Error) -> Self {
        StorageError::Serialization(e.to_string())
    }
}

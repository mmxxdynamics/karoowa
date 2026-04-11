//! Crate-wide error types for `karoowa-core`.

use karoowa_crypto::{HashError, SignatureError};

/// Top-level error type for `karoowa-core` operations.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// A cryptographic operation failed (hash parsing, signature verification).
    #[error(transparent)]
    Crypto(#[from] CryptoError),

    /// A block failed validation.
    #[error("block validation error: {0}")]
    BlockValidation(String),

    /// A transaction failed validation.
    #[error("transaction validation error: {0}")]
    TransactionValidation(String),

    /// Genesis configuration is invalid.
    #[error("genesis error: {0}")]
    Genesis(String),

    /// Serialization / deserialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Wrapper for crypto-layer errors so they integrate into [`CoreError`].
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error(transparent)]
    Hash(#[from] HashError),
    #[error(transparent)]
    Signature(#[from] SignatureError),
}

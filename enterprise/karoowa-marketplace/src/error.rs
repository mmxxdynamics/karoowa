//! Marketplace error types.

use karoowa_crypto::Hash;

/// Errors produced by marketplace operations.
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    /// Filesystem failure while reading an attestation file.
    #[error("marketplace io error: {0}")]
    Io(#[from] std::io::Error),

    /// The attestation file is syntactically malformed.
    #[error("malformed attestation: {0}")]
    Malformed(String),

    /// The attestation is signed by a vendor key other than the
    /// compiled-in Karoowa key.
    #[error("attestation signed by unknown vendor")]
    UnknownVendor,

    /// The attestation signature did not verify against the
    /// canonical payload bytes.
    #[error("attestation signature invalid")]
    BadAttestation,

    /// The attestation has expired.
    #[error("attestation expired at {expired_at}, now is {now}")]
    Expired { expired_at: u64, now: u64 },

    /// An agent with this id is already registered.
    #[error("duplicate agent id: {0}")]
    Duplicate(String),

    /// The bytecode handed to the loader does not hash to the
    /// content hash in the attestation.
    #[error("bytecode content hash mismatch: expected {expected}, got {actual}")]
    ContentMismatch { expected: Hash, actual: Hash },
}

//! License error types.

/// Errors produced by license parsing and verification.
#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    /// The license file could not be read from disk.
    #[error("failed to read license file: {0}")]
    Io(#[from] std::io::Error),

    /// The license file is syntactically malformed.
    #[error("malformed license file: {0}")]
    Malformed(String),

    /// The license is signed by a public key that is not the
    /// compiled-in vendor key.
    #[error("license signed by unknown vendor key")]
    UnknownVendor,

    /// The signature did not verify against the payload bytes.
    #[error("license signature verification failed")]
    BadSignature,

    /// The license declares an edition other than `"enterprise"`.
    #[error("license declares wrong edition: {0}")]
    WrongEdition(String),

    /// The license has expired.
    #[error("license expired at {expired_at}, now is {now}")]
    Expired { expired_at: u64, now: u64 },
}

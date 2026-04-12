//! HSM error types.

/// Errors produced by HSM operations.
#[derive(Debug, thiserror::Error)]
pub enum HsmError {
    /// Filesystem failure while reading or writing the store.
    #[error("hsm io error: {0}")]
    Io(#[from] std::io::Error),

    /// The store file is syntactically malformed.
    #[error("malformed hsm store: {0}")]
    Malformed(String),

    /// The key id does not exist in this HSM.
    #[error("unknown hsm key id: {0}")]
    UnknownKey(String),

    /// A key with this id already exists.
    #[error("duplicate hsm key id: {0}")]
    DuplicateKey(String),

    /// The HSM backend is unavailable (e.g. network to AWS CloudHSM
    /// is down, YubiHSM is disconnected). Not used by SoftHsm.
    #[error("hsm backend unavailable: {0}")]
    Unavailable(String),

    /// Internal invariant violated.
    #[error("hsm internal error: {0}")]
    Internal(String),
}

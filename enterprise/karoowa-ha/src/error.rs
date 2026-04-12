//! HA coordinator error types.

/// Errors produced by lease backends and the HA coordinator.
#[derive(Debug, thiserror::Error)]
pub enum HaError {
    /// Another node currently holds a non-expired lease.
    #[error("lease held by another node")]
    Contention,

    /// The caller is not the current holder, or the lease has
    /// already expired from under them.
    #[error("lease lost")]
    LeaseLost,

    /// The backend is unreachable (network down, DB offline).
    #[error("ha backend unavailable: {0}")]
    Unavailable(String),

    /// Internal invariant violated.
    #[error("ha internal error: {0}")]
    Internal(String),
}

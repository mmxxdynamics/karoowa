//! Audit log error types.

/// Errors produced by audit log operations.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    /// Filesystem failure while reading or writing the log.
    #[error("audit log io error: {0}")]
    Io(#[from] std::io::Error),

    /// A log record could not be parsed as a valid event.
    #[error("malformed audit record: {0}")]
    Malformed(String),

    /// The hash chain is broken at the given sequence number —
    /// either the previous hash doesn't match or the event's own
    /// hash doesn't match its content.
    #[error("audit log hash chain broken at sequence {sequence}")]
    ChainBroken { sequence: u64 },

    /// Internal invariant violated.
    #[error("audit log internal error: {0}")]
    Internal(String),
}

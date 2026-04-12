//! RBAC error types.

/// Errors produced by RBAC policy loading and evaluation.
#[derive(Debug, thiserror::Error)]
pub enum RbacError {
    /// The policy file could not be read.
    #[error("failed to read RBAC policy: {0}")]
    Io(#[from] std::io::Error),

    /// The policy file is syntactically malformed.
    #[error("malformed RBAC policy: {0}")]
    Malformed(String),
}

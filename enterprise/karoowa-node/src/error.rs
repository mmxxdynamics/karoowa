//! Enterprise wrapper error types.

/// Errors from enterprise context bootstrap and runtime.
#[derive(Debug, thiserror::Error)]
pub enum EnterpriseError {
    /// Filesystem failure reading a config file.
    #[error("enterprise io error: {0}")]
    Io(#[from] std::io::Error),

    /// License loading / verification failed.
    #[error("license error: {0}")]
    License(String),

    /// Audit log could not be opened.
    #[error("audit log error: {0}")]
    Audit(String),

    /// RBAC policy failed to load.
    #[error("rbac error: {0}")]
    Rbac(String),

    /// HSM bootstrap or operation failed.
    #[error("hsm error: {0}")]
    Hsm(String),

    /// Marketplace attestation failed verification.
    #[error("marketplace error: {0}")]
    Marketplace(String),

    /// An enterprise feature was requested but the license does
    /// not enable it.
    #[error("feature not licensed: {0}")]
    FeatureNotLicensed(String),
}

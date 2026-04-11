//! SDK error types.

/// Errors produced by SDK operations.
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    /// HTTP request failed.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON-RPC returned an error response.
    #[error("rpc error ({code}): {message}")]
    Rpc { code: i64, message: String },

    /// Failed to parse a response field.
    #[error("parse error: {0}")]
    Parse(String),

    /// Transaction construction or signing failed.
    #[error("transaction error: {0}")]
    Transaction(String),
}

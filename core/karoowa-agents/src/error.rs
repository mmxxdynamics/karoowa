//! Agent error types.

/// Errors produced by agent operations.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// LLM provider returned an error.
    #[error("provider error: {0}")]
    Provider(String),

    /// HTTP request to the provider failed.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// Tool execution failed.
    #[error("tool error: {0}")]
    Tool(String),

    /// Agent configuration is invalid.
    #[error("config error: {0}")]
    Config(String),

    /// Memory store error.
    #[error("memory error: {0}")]
    Memory(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),
}

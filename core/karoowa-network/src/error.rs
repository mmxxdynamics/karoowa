//! Network error types.

/// Errors produced by networking operations.
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    /// Failed to start listening on the configured address.
    #[error("listen error: {0}")]
    Listen(String),

    /// Failed to dial a peer.
    #[error("dial error: {0}")]
    Dial(String),

    /// Gossipsub publish failed.
    #[error("publish error: {0}")]
    Publish(String),

    /// Serialization / deserialization of a network message failed.
    #[error("codec error: {0}")]
    Codec(String),

    /// The network event loop is not running.
    #[error("network not running")]
    NotRunning,

    /// Transport or swarm construction failed.
    #[error("transport error: {0}")]
    Transport(String),
}

impl From<bincode::Error> for NetworkError {
    fn from(e: bincode::Error) -> Self {
        NetworkError::Codec(e.to_string())
    }
}

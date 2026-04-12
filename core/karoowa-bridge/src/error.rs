//! Bridge error types.

use karoowa_crypto::Hash;

/// Errors produced by bridge operations.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    /// The packet was already processed (replay protection).
    #[error("packet {0} already processed")]
    DuplicatePacket(Hash),

    /// The packet's source-chain commitment proof failed verification.
    #[error("commitment proof invalid: {0}")]
    CommitmentInvalid(String),

    /// Insufficient balance for the requested operation.
    #[error("insufficient balance: needed {needed}, available {available}")]
    InsufficientBalance { needed: u64, available: u64 },

    /// Channel is not in the expected state.
    #[error("channel state mismatch: expected {expected}, got {got}")]
    ChannelStateMismatch { expected: String, got: String },

    /// Packet timed out before being relayed.
    #[error("packet timed out: source height {source_height}, current height {current}")]
    PacketTimeout { source_height: u64, current: u64 },

    /// Light client error during proof verification.
    #[error("light client error: {0}")]
    LightClient(String),

    /// Relayer encountered an internal inconsistency.
    #[error("relayer error: {0}")]
    Relayer(String),
}

impl From<karoowa_light::LightClientError> for BridgeError {
    fn from(e: karoowa_light::LightClientError) -> Self {
        BridgeError::LightClient(e.to_string())
    }
}

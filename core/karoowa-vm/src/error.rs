//! VM error types.

/// Errors produced by the WASM VM.
#[derive(Debug, thiserror::Error)]
pub enum VmError {
    /// WASM compilation failed.
    #[error("compilation error: {0}")]
    Compilation(String),

    /// WASM instantiation failed.
    #[error("instantiation error: {0}")]
    Instantiation(String),

    /// Contract execution trapped (panic, unreachable, etc.).
    #[error("trap: {0}")]
    Trap(String),

    /// Contract ran out of gas (fuel exhausted).
    #[error("out of gas: used {used}, limit {limit}")]
    OutOfGas { used: u64, limit: u64 },

    /// Contract explicitly reverted with a reason.
    #[error("revert: {0}")]
    Revert(String),

    /// Host function error.
    #[error("host error: {0}")]
    Host(String),

    /// Storage error during contract execution.
    #[error("storage error: {0}")]
    Storage(String),

    /// Invalid contract bytecode.
    #[error("invalid bytecode: {0}")]
    InvalidBytecode(String),
}

impl From<wasmtime::Error> for VmError {
    fn from(e: wasmtime::Error) -> Self {
        VmError::Trap(e.to_string())
    }
}

impl From<karoowa_storage::StorageError> for VmError {
    fn from(e: karoowa_storage::StorageError) -> Self {
        VmError::Storage(e.to_string())
    }
}

//! Execution context — binds a contract call to blockchain state.
//!
//! Provides storage isolation per contract address by prefixing all
//! storage keys with the contract address.

use karoowa_crypto::{sha3_256, Address};
use karoowa_storage::{StateStore, StorageError};

/// Execution context for a contract call.
pub struct ExecutionContext<'a, S: StateStore> {
    /// The contract being executed.
    pub contract_address: Address,
    /// The caller (EOA or calling contract).
    pub caller: Address,
    /// Value sent with the call.
    pub value: u64,
    /// Current block height.
    pub block_height: u64,
    /// Storage backend (shared, but reads/writes are scoped to contract_address).
    pub storage: &'a S,
    /// Reentrancy guard — set of contract addresses currently in the call stack.
    pub call_stack: Vec<Address>,
}

impl<'a, S: StateStore> ExecutionContext<'a, S> {
    /// Read a storage slot for the current contract.
    pub fn storage_read(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let slot_hash = sha3_256(key);
        self.storage.get_storage(&self.contract_address, &slot_hash)
    }

    /// Write a storage slot for the current contract.
    pub fn storage_write(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let slot_hash = sha3_256(key);
        self.storage
            .put_storage(&self.contract_address, &slot_hash, value)
    }

    /// Check if calling a contract would be reentrant.
    pub fn is_reentrant(&self, target: &Address) -> bool {
        self.call_stack.contains(target)
    }
}

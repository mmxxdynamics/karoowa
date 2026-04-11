//! Storage trait definitions.
//!
//! These traits define the interface that any storage backend must implement.
//! The RocksDB implementation lives in [`super::rocks`]. Alternative backends
//! (e.g. `sled`, `redb`, in-memory for tests) can implement the same traits.

use karoowa_core::{Account, Block, Receipt, StateDiff};
use karoowa_crypto::{Address, Hash};

use crate::StorageError;

/// Persistent storage for blocks.
pub trait BlockStore: Send + Sync {
    /// Store a block. The implementation must index it by both hash and height.
    fn put_block(&self, block: &Block) -> Result<(), StorageError>;

    /// Retrieve a block by its hash.
    fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>, StorageError>;

    /// Retrieve a block by its height.
    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>, StorageError>;

    /// Return the latest (highest) block, or `None` if the store is empty.
    fn head(&self) -> Result<Option<Block>, StorageError>;

    /// Return the height of the latest block, or `None` if empty.
    fn head_height(&self) -> Result<Option<u64>, StorageError>;
}

/// Persistent storage for account state.
pub trait StateStore: Send + Sync {
    /// Get the account state for an address.
    fn get_account(&self, address: &Address) -> Result<Option<Account>, StorageError>;

    /// Put (create or overwrite) account state.
    fn put_account(&self, address: &Address, account: &Account) -> Result<(), StorageError>;

    /// Read a storage slot for a contract.
    fn get_storage(&self, address: &Address, key: &Hash) -> Result<Option<Vec<u8>>, StorageError>;

    /// Write a storage slot for a contract.
    fn put_storage(&self, address: &Address, key: &Hash, value: &[u8]) -> Result<(), StorageError>;

    /// Apply a [`StateDiff`] atomically. Returns the new state root hash.
    ///
    /// The state root is a deterministic hash over the complete state. In M1
    /// this is a placeholder (hash of the serialized diff); a proper Merkle
    /// Patricia trie root comes later.
    fn commit(&self, diff: &StateDiff) -> Result<Hash, StorageError>;
}

/// Persistent storage for transaction receipts.
pub trait ReceiptStore: Send + Sync {
    /// Store a receipt, indexed by its transaction hash.
    fn put_receipt(&self, receipt: &Receipt) -> Result<(), StorageError>;

    /// Retrieve a receipt by transaction hash.
    fn get_receipt_by_tx_hash(&self, tx_hash: &Hash) -> Result<Option<Receipt>, StorageError>;
}

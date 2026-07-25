//! RocksDB-backed implementation of the storage traits.
//!
//! Uses column families for logical separation:
//! - `blocks` — block data keyed by hash
//! - `block_height_index` — block hash keyed by height (u64 big-endian)
//! - `state_accounts` — account data keyed by address bytes
//! - `state_storage` — contract storage keyed by `address || slot_hash`
//! - `receipts` — receipt data keyed by tx hash
//! - `tx_index` — block hash keyed by tx hash (tx → block lookup)
//! - `meta` — singleton metadata (e.g. head height)

use karoowa_core::{Account, AccountChange, Block, Receipt, StateDiff};
use karoowa_crypto::{sha3_256, Address, Hash};
use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};
use std::path::Path;
use std::sync::Arc;

use crate::error::StorageError;
use crate::traits::{BlockStore, ReceiptStore, StateStore};

/// Column family names.
const CF_BLOCKS: &str = "blocks";
const CF_BLOCK_HEIGHT: &str = "block_height_index";
const CF_ACCOUNTS: &str = "state_accounts";
const CF_STORAGE: &str = "state_storage";
const CF_RECEIPTS: &str = "receipts";
const CF_TX_INDEX: &str = "tx_index";
const CF_META: &str = "meta";

const META_HEAD_HEIGHT: &[u8] = b"head_height";

/// RocksDB-backed storage engine.
///
/// This struct implements all three storage traits ([`BlockStore`],
/// [`StateStore`], [`ReceiptStore`]) against a single RocksDB instance
/// with column families.
#[derive(Clone)]
pub struct RocksStorage {
    db: Arc<DB>,
}

impl RocksStorage {
    /// Open (or create) a RocksDB database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_opts = Options::default();
        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_BLOCKS, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_BLOCK_HEIGHT, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_ACCOUNTS, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_STORAGE, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_RECEIPTS, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_TX_INDEX, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_META, cf_opts),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;
        Ok(RocksStorage { db: Arc::new(db) })
    }

    /// Helper: get a column family handle (panics if CF doesn't exist, which
    /// is a programming error since we create them at open time).
    fn cf(&self, name: &str) -> &rocksdb::ColumnFamily {
        self.db
            .cf_handle(name)
            .unwrap_or_else(|| panic!("missing column family: {name}"))
    }

    /// Encode a u64 as 8 big-endian bytes (for ordered height keys).
    fn height_key(height: u64) -> [u8; 8] {
        height.to_be_bytes()
    }

    /// Decode a u64 from 8 big-endian bytes.
    fn decode_height(bytes: &[u8]) -> u64 {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(bytes);
        u64::from_be_bytes(buf)
    }

    /// Build a composite key for contract storage: `address_bytes || slot_hash_bytes`.
    fn storage_key(address: &Address, slot: &Hash) -> Vec<u8> {
        let mut key = Vec::with_capacity(52); // 20 + 32
        key.extend_from_slice(address.as_bytes());
        key.extend_from_slice(slot.as_bytes());
        key
    }
}

// ---------------------------------------------------------------------------
// BlockStore
// ---------------------------------------------------------------------------

impl BlockStore for RocksStorage {
    fn put_block(&self, block: &Block) -> Result<(), StorageError> {
        let hash = block.hash();
        let height = block.height();
        let block_bytes = bincode::serialize(block)?;

        let mut batch = WriteBatch::default();

        // Store block by hash.
        batch.put_cf(self.cf(CF_BLOCKS), hash.as_bytes(), &block_bytes);

        // Index: height → hash.
        batch.put_cf(
            self.cf(CF_BLOCK_HEIGHT),
            Self::height_key(height),
            hash.as_bytes(),
        );

        // Index: tx_hash → block_hash for each transaction.
        for tx in &block.transactions {
            batch.put_cf(self.cf(CF_TX_INDEX), tx.hash().as_bytes(), hash.as_bytes());
        }

        // Update head height if this block is higher than current.
        let should_update = match self.head_height()? {
            None => true,
            Some(current) => height > current,
        };
        if should_update {
            batch.put_cf(self.cf(CF_META), META_HEAD_HEIGHT, Self::height_key(height));
        }

        self.db.write(batch)?;
        Ok(())
    }

    fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>, StorageError> {
        match self.db.get_cf(self.cf(CF_BLOCKS), hash.as_bytes())? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>, StorageError> {
        // Look up hash from height index, then fetch block by hash.
        match self
            .db
            .get_cf(self.cf(CF_BLOCK_HEIGHT), Self::height_key(height))?
        {
            Some(hash_bytes) => {
                let hash = Hash::from_bytes(
                    hash_bytes
                        .as_slice()
                        .try_into()
                        .map_err(|_| StorageError::Consistency("invalid hash in index".into()))?,
                );
                self.get_block_by_hash(&hash)
            }
            None => Ok(None),
        }
    }

    fn head(&self) -> Result<Option<Block>, StorageError> {
        match self.head_height()? {
            Some(h) => self.get_block_by_height(h),
            None => Ok(None),
        }
    }

    fn head_height(&self) -> Result<Option<u64>, StorageError> {
        match self.db.get_cf(self.cf(CF_META), META_HEAD_HEIGHT)? {
            Some(bytes) => Ok(Some(Self::decode_height(&bytes))),
            None => Ok(None),
        }
    }

    fn get_block_hash_by_tx(&self, tx_hash: &Hash) -> Result<Option<Hash>, StorageError> {
        match self.db.get_cf(self.cf(CF_TX_INDEX), tx_hash.as_bytes())? {
            Some(hash_bytes) => Ok(Some(Hash::from_bytes(
                hash_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| StorageError::Consistency("invalid hash in tx index".into()))?,
            ))),
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// StateStore
// ---------------------------------------------------------------------------

impl StateStore for RocksStorage {
    fn get_account(&self, address: &Address) -> Result<Option<Account>, StorageError> {
        match self.db.get_cf(self.cf(CF_ACCOUNTS), address.as_bytes())? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    fn put_account(&self, address: &Address, account: &Account) -> Result<(), StorageError> {
        let bytes = bincode::serialize(account)?;
        self.db
            .put_cf(self.cf(CF_ACCOUNTS), address.as_bytes(), bytes)?;
        Ok(())
    }

    fn get_storage(&self, address: &Address, key: &Hash) -> Result<Option<Vec<u8>>, StorageError> {
        let composite = Self::storage_key(address, key);
        match self.db.get_cf(self.cf(CF_STORAGE), &composite)? {
            Some(v) => Ok(Some(v)),
            None => Ok(None),
        }
    }

    fn put_storage(&self, address: &Address, key: &Hash, value: &[u8]) -> Result<(), StorageError> {
        let composite = Self::storage_key(address, key);
        self.db.put_cf(self.cf(CF_STORAGE), &composite, value)?;
        Ok(())
    }

    fn commit(&self, diff: &StateDiff) -> Result<Hash, StorageError> {
        let mut batch = WriteBatch::default();

        for (address, change) in &diff.accounts {
            match change {
                AccountChange::Modified(account) => {
                    let bytes = bincode::serialize(account)?;
                    batch.put_cf(self.cf(CF_ACCOUNTS), address.as_bytes(), bytes);
                }
                AccountChange::Deleted => {
                    batch.delete_cf(self.cf(CF_ACCOUNTS), address.as_bytes());
                }
            }
        }

        for (address, slots) in &diff.storage {
            for (slot, value) in slots {
                let composite = Self::storage_key(address, slot);
                if value.is_empty() {
                    batch.delete_cf(self.cf(CF_STORAGE), &composite);
                } else {
                    batch.put_cf(self.cf(CF_STORAGE), &composite, value);
                }
            }
        }

        self.db.write(batch)?;

        // Placeholder state root: hash the serialized diff.
        // A proper Merkle Patricia trie root is a future improvement.
        let diff_bytes = bincode::serialize(diff)?;
        Ok(sha3_256(&diff_bytes))
    }
}

// ---------------------------------------------------------------------------
// ReceiptStore
// ---------------------------------------------------------------------------

impl ReceiptStore for RocksStorage {
    fn put_receipt(&self, receipt: &Receipt) -> Result<(), StorageError> {
        let bytes = bincode::serialize(receipt)?;
        self.db
            .put_cf(self.cf(CF_RECEIPTS), receipt.tx_hash.as_bytes(), bytes)?;
        Ok(())
    }

    fn get_receipt_by_tx_hash(&self, tx_hash: &Hash) -> Result<Option<Receipt>, StorageError> {
        match self.db.get_cf(self.cf(CF_RECEIPTS), tx_hash.as_bytes())? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }
}

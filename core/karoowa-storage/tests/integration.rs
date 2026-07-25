//! Integration tests for RocksStorage.
//!
//! Each test gets a fresh temporary directory via `tempfile` so tests don't
//! interfere with each other.

use karoowa_core::*;
use karoowa_crypto::*;
use karoowa_storage::*;
use tempfile::TempDir;

fn open_temp_db() -> (RocksStorage, TempDir) {
    let dir = TempDir::new().unwrap();
    let storage = RocksStorage::open(dir.path()).unwrap();
    (storage, dir)
}

fn test_keypair() -> Keypair {
    Keypair::from_seed(&[1u8; 32])
}

fn make_tx(kp: &Keypair, nonce: u64) -> Transaction {
    let to = Address::from_public_key(&[2u8; 32]);
    Transaction::sign_transfer(kp, to, 100, nonce, 1, 21000, 1)
}

fn make_block(height: u64, parent: Hash, txs: Vec<Transaction>) -> Block {
    let proposer = Address::from_public_key(&[99u8; 32]);
    BlockBuilder::new(parent, height, 1700000000 + height, proposer)
        .transactions(txs)
        .build()
}

// ---------------------------------------------------------------------------
// BlockStore tests
// ---------------------------------------------------------------------------

#[test]
fn put_and_get_block_by_hash() {
    let (db, _dir) = open_temp_db();
    let kp = test_keypair();
    let block = make_block(0, Hash::ZERO, vec![make_tx(&kp, 0)]);
    let hash = block.hash();

    db.put_block(&block).unwrap();
    let retrieved = db.get_block_by_hash(&hash).unwrap().unwrap();
    assert_eq!(retrieved.hash(), hash);
    assert_eq!(retrieved.transactions.len(), 1);
}

#[test]
fn put_and_get_block_by_height() {
    let (db, _dir) = open_temp_db();
    let kp = test_keypair();
    let block = make_block(5, Hash::ZERO, vec![make_tx(&kp, 0)]);

    db.put_block(&block).unwrap();
    let retrieved = db.get_block_by_height(5).unwrap().unwrap();
    assert_eq!(retrieved.hash(), block.hash());
}

#[test]
fn get_nonexistent_block_returns_none() {
    let (db, _dir) = open_temp_db();
    assert!(db.get_block_by_hash(&Hash::ZERO).unwrap().is_none());
    assert!(db.get_block_by_height(999).unwrap().is_none());
}

#[test]
fn head_tracks_highest_block() {
    let (db, _dir) = open_temp_db();
    let kp = test_keypair();

    assert!(db.head().unwrap().is_none());
    assert!(db.head_height().unwrap().is_none());

    let b0 = make_block(0, Hash::ZERO, vec![make_tx(&kp, 0)]);
    db.put_block(&b0).unwrap();
    assert_eq!(db.head_height().unwrap(), Some(0));

    let b1 = make_block(1, b0.hash(), vec![make_tx(&kp, 1)]);
    db.put_block(&b1).unwrap();
    assert_eq!(db.head_height().unwrap(), Some(1));
    assert_eq!(db.head().unwrap().unwrap().hash(), b1.hash());
}

#[test]
fn chain_of_blocks() {
    let (db, _dir) = open_temp_db();
    let kp = test_keypair();

    let mut parent = Hash::ZERO;
    for i in 0..10 {
        let block = make_block(i, parent, vec![make_tx(&kp, i)]);
        parent = block.hash();
        db.put_block(&block).unwrap();
    }

    assert_eq!(db.head_height().unwrap(), Some(9));

    // Verify the chain links.
    for i in 1..10 {
        let block = db.get_block_by_height(i).unwrap().unwrap();
        let prev = db.get_block_by_height(i - 1).unwrap().unwrap();
        assert_eq!(block.header.parent_hash, prev.hash());
    }
}

// ---------------------------------------------------------------------------
// StateStore tests
// ---------------------------------------------------------------------------

#[test]
fn put_and_get_account() {
    let (db, _dir) = open_temp_db();
    let addr = Address::from_public_key(&[1u8; 32]);
    let account = Account {
        balance: 1_000_000,
        nonce: 5,
        ..Account::default()
    };

    db.put_account(&addr, &account).unwrap();
    let retrieved = db.get_account(&addr).unwrap().unwrap();
    assert_eq!(retrieved.balance, 1_000_000);
    assert_eq!(retrieved.nonce, 5);
}

#[test]
fn get_nonexistent_account_returns_none() {
    let (db, _dir) = open_temp_db();
    let addr = Address::from_public_key(&[99u8; 32]);
    assert!(db.get_account(&addr).unwrap().is_none());
}

#[test]
fn put_and_get_storage_slot() {
    let (db, _dir) = open_temp_db();
    let addr = Address::from_public_key(&[1u8; 32]);
    let slot = sha3_256(b"slot_0");
    let value = b"hello storage";

    db.put_storage(&addr, &slot, value).unwrap();
    let retrieved = db.get_storage(&addr, &slot).unwrap().unwrap();
    assert_eq!(retrieved, value);
}

#[test]
fn commit_state_diff() {
    let (db, _dir) = open_temp_db();
    let from = Address::from_public_key(&[1u8; 32]);
    let to = Address::from_public_key(&[2u8; 32]);

    // Set up initial state.
    let from_acct = Account {
        balance: 1000,
        ..Account::default()
    };
    let to_acct = Account::default();
    db.put_account(&from, &from_acct).unwrap();
    db.put_account(&to, &to_acct).unwrap();

    // Build and commit a diff.
    let mut diff = StateDiff::new();
    diff.apply_transfer(from, to, 300, &from_acct, &to_acct);

    let state_root = db.commit(&diff).unwrap();
    assert_ne!(state_root, Hash::ZERO);

    // Verify committed state.
    let from_after = db.get_account(&from).unwrap().unwrap();
    let to_after = db.get_account(&to).unwrap().unwrap();
    assert_eq!(from_after.balance, 700);
    assert_eq!(from_after.nonce, 1);
    assert_eq!(to_after.balance, 300);
}

// ---------------------------------------------------------------------------
// ReceiptStore tests
// ---------------------------------------------------------------------------

#[test]
fn put_and_get_receipt() {
    let (db, _dir) = open_temp_db();
    let receipt = Receipt {
        tx_hash: sha3_256(b"tx-1"),
        status: TxStatus::Success,
        gas_used: 21000,
        logs: vec![Log {
            address: Address::ZERO,
            topics: vec![sha3_256(b"Transfer")],
            data: vec![1, 2, 3],
        }],
        output: vec![],
    };

    db.put_receipt(&receipt).unwrap();
    let retrieved = db
        .get_receipt_by_tx_hash(&receipt.tx_hash)
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.tx_hash, receipt.tx_hash);
    assert_eq!(retrieved.gas_used, 21000);
    assert_eq!(retrieved.logs.len(), 1);
}

#[test]
fn get_nonexistent_receipt_returns_none() {
    let (db, _dir) = open_temp_db();
    assert!(db.get_receipt_by_tx_hash(&Hash::ZERO).unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Soak test
// ---------------------------------------------------------------------------

#[test]
fn soak_write_and_read_many_blocks() {
    let (db, _dir) = open_temp_db();
    let kp = test_keypair();
    let block_count = 100;

    let mut parent = Hash::ZERO;
    let mut hashes = Vec::with_capacity(block_count);

    for i in 0..block_count as u64 {
        let block = make_block(i, parent, vec![make_tx(&kp, i)]);
        parent = block.hash();
        hashes.push(parent);
        db.put_block(&block).unwrap();
    }

    assert_eq!(db.head_height().unwrap(), Some(block_count as u64 - 1));

    // Read random blocks by height and verify hash.
    for i in [0, 1, 49, 50, 98, 99] {
        let block = db.get_block_by_height(i).unwrap().unwrap();
        assert_eq!(block.hash(), hashes[i as usize]);
    }

    // Read by hash.
    for (i, hash) in hashes.iter().enumerate().take(10) {
        let block = db.get_block_by_hash(hash).unwrap().unwrap();
        assert_eq!(block.height(), i as u64);
    }
}

#[test]
fn tx_index_resolves_containing_block() {
    let (db, _dir) = open_temp_db();
    let kp = test_keypair();
    let tx = make_tx(&kp, 0);
    let tx_hash = tx.hash();
    let block = make_block(3, Hash::ZERO, vec![tx]);

    db.put_block(&block).unwrap();

    let resolved = db.get_block_hash_by_tx(&tx_hash).unwrap();
    assert_eq!(resolved, Some(block.hash()));

    let unknown = Hash::from_bytes([7u8; 32]);
    assert_eq!(db.get_block_hash_by_tx(&unknown).unwrap(), None);
}

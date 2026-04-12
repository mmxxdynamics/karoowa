//! Integration test for `ExecutionContext` storage isolation.
//!
//! Verifies the per-contract storage scoping and the reentrancy guard.
//! Uses a real RocksStorage backed by a tempdir since `StateStore` is
//! the only trait `ExecutionContext` is generic over and we want the
//! test to exercise the production backend.

use karoowa_crypto::Address;
use karoowa_storage::RocksStorage;
use karoowa_vm::ExecutionContext;

fn addr(b: u8) -> Address {
    Address::from_public_key(&[b; 32])
}

#[test]
fn storage_isolated_per_contract_and_reentrancy_guard() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = RocksStorage::open(tmp.path()).unwrap();

    let contract_a = addr(0xA1);
    let contract_b = addr(0xB1);

    let ctx_a = ExecutionContext {
        contract_address: contract_a,
        caller: addr(0x01),
        value: 0,
        block_height: 1,
        storage: &storage,
        call_stack: vec![contract_a],
    };

    // Write under contract A.
    ctx_a.storage_write(b"counter", b"\x01\x02").unwrap();
    assert_eq!(
        ctx_a.storage_read(b"counter").unwrap(),
        Some(b"\x01\x02".to_vec())
    );

    // Contract B reading the same logical key must not see A's value —
    // the slot hash is the same, but the (address, key) tuple differs.
    let ctx_b = ExecutionContext {
        contract_address: contract_b,
        caller: addr(0x01),
        value: 0,
        block_height: 1,
        storage: &storage,
        call_stack: vec![contract_b],
    };
    assert_eq!(ctx_b.storage_read(b"counter").unwrap(), None);

    // Reentrancy guard: ctx_a's call_stack contains contract_a, so
    // calling back into A from A must be flagged.
    assert!(ctx_a.is_reentrant(&contract_a));
    assert!(!ctx_a.is_reentrant(&contract_b));
}

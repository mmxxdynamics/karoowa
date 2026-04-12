//! Property-based tests for the WASM VM's adversarial input surface.
//!
//! A malicious user can upload arbitrary bytecode through the
//! contract-deployment path, so wasmtime's validator is a load-bearing
//! gate. These tests assert that feeding random or near-valid bytes to
//! `ContractExecutor::deploy` returns a clean `Err(VmError)` rather than
//! panicking the node process — the single most important "no DoS" rule
//! for the VM.

use karoowa_crypto::Address;
use karoowa_vm::ContractExecutor;
use proptest::prelude::*;

// 32 cases per property — keeps CI fast while still exercising a wide
// random sample through wasmtime's validator.
const CASES: u32 = 32;

const WASM_MAGIC: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

fn deployer() -> Address {
    Address::from_public_key(&[7u8; 32])
}

fn run_deploy(executor: &ContractExecutor, bytes: &[u8]) {
    // Low gas limit — even if wasmtime *does* admit the bytes, we don't
    // want the fuzz run to burn CPU on real execution.
    let _ = executor.deploy(bytes, &[], 1_000, deployer(), 0);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    /// Arbitrary junk bytes must never panic deploy(). A panic here would
    /// crash the node on malicious contract upload.
    #[test]
    fn deploy_junk_bytes_never_panics(
        bytes in proptest::collection::vec(any::<u8>(), 0..512),
    ) {
        let executor = ContractExecutor::new().unwrap();
        run_deploy(&executor, &bytes);
    }

    /// Bytes prefixed with the WASM magic header exercise wasmtime's
    /// module validator (which is a deeper code path than the "not a
    /// module at all" rejection from random bytes). Must not panic.
    #[test]
    fn deploy_wasm_magic_plus_junk_never_panics(
        tail in proptest::collection::vec(any::<u8>(), 0..512),
    ) {
        let mut bytes = Vec::with_capacity(8 + tail.len());
        bytes.extend_from_slice(&WASM_MAGIC);
        bytes.extend_from_slice(&tail);
        let executor = ContractExecutor::new().unwrap();
        run_deploy(&executor, &bytes);
    }

    /// Empty and single-byte inputs — the smallest possible inputs
    /// sometimes reveal bounds-checking bugs that larger inputs mask.
    #[test]
    fn deploy_tiny_inputs_never_panic(len in 0usize..=4) {
        let bytes = vec![0u8; len];
        let executor = ContractExecutor::new().unwrap();
        run_deploy(&executor, &bytes);
    }
}

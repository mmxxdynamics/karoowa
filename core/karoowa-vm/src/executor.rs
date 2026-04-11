//! Contract executor — high-level API for deploying and calling contracts.
//!
//! Wraps the low-level [`WasmVm`] with blockchain-aware features:
//! - Contract deployment (store bytecode, derive address)
//! - Contract invocation with storage persistence
//! - Receipt enrichment with gas, events, and return data
//! - Reentrancy protection

use karoowa_core::{Log, Receipt, TxStatus};
use karoowa_crypto::{sha3_256, Address, Hash};

use crate::error::VmError;
use crate::vm::{ExecutionResult, VmConfig, WasmVm};

/// A deployed contract in the system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Contract {
    /// Contract address.
    pub address: Address,
    /// Deployer address.
    pub deployer: Address,
    /// WASM bytecode.
    pub bytecode: Vec<u8>,
    /// Block height at which the contract was deployed.
    pub deployed_at: u64,
}

/// The contract executor.
pub struct ContractExecutor {
    vm: WasmVm,
}

impl ContractExecutor {
    /// Create a new executor with default VM config.
    pub fn new() -> Result<Self, VmError> {
        Ok(ContractExecutor {
            vm: WasmVm::new(VmConfig::default())?,
        })
    }

    /// Create with a custom VM config.
    pub fn with_config(config: VmConfig) -> Result<Self, VmError> {
        Ok(ContractExecutor {
            vm: WasmVm::new(config)?,
        })
    }

    /// Derive a contract address from the deployer address and nonce.
    pub fn derive_address(deployer: &Address, nonce: u64) -> Address {
        let mut input = Vec::new();
        input.extend_from_slice(deployer.as_bytes());
        input.extend_from_slice(&nonce.to_be_bytes());
        let hash = sha3_256(&input);
        // Take last 20 bytes of hash as address.
        let mut addr_bytes = [0u8; 20];
        addr_bytes.copy_from_slice(&hash.as_bytes()[12..32]);
        Address::from_bytes(addr_bytes)
    }

    /// Deploy a contract. Returns the contract metadata and the execution result
    /// (from running the constructor, if the WASM exports a "deploy" function).
    pub fn deploy(
        &self,
        bytecode: &[u8],
        constructor_args: &[u8],
        gas_limit: u64,
        deployer: Address,
        nonce: u64,
    ) -> Result<(Contract, ExecutionResult), VmError> {
        let contract_address = Self::derive_address(&deployer, nonce);

        // Try running the "deploy" constructor if it exists.
        let result = self.vm.execute(
            bytecode,
            "deploy",
            constructor_args,
            gas_limit,
            deployer,
            0,
            contract_address,
        );

        let exec_result = match result {
            Ok(r) => r,
            Err(VmError::Trap(msg)) if msg.contains("not found") => {
                // No deploy function — that's ok, just store the bytecode.
                ExecutionResult {
                    output: vec![],
                    gas_used: 0,
                    events: vec![],
                    success: true,
                    revert_reason: None,
                }
            }
            Err(e) => return Err(e),
        };

        let contract = Contract {
            address: contract_address,
            deployer,
            bytecode: bytecode.to_vec(),
            deployed_at: 0, // Set by the caller.
        };

        Ok((contract, exec_result))
    }

    /// Call a deployed contract.
    pub fn call(
        &self,
        contract: &Contract,
        input: &[u8],
        gas_limit: u64,
        caller: Address,
        value: u64,
    ) -> Result<ExecutionResult, VmError> {
        self.vm.execute(
            &contract.bytecode,
            "call",
            input,
            gas_limit,
            caller,
            value,
            contract.address,
        )
    }

    /// Build a receipt from an execution result.
    pub fn build_receipt(
        tx_hash: Hash,
        result: &ExecutionResult,
        _contract_address: Option<Address>,
    ) -> Receipt {
        let logs: Vec<Log> = result
            .events
            .iter()
            .map(|e| Log {
                address: e.address,
                topics: e.topics.clone(),
                data: e.data.clone(),
            })
            .collect();

        Receipt {
            tx_hash,
            status: if result.success {
                TxStatus::Success
            } else {
                TxStatus::Reverted
            },
            gas_used: result.gas_used,
            logs,
            output: result.output.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_address_is_deterministic() {
        let deployer = Address::from_public_key(&[1u8; 32]);
        let addr1 = ContractExecutor::derive_address(&deployer, 0);
        let addr2 = ContractExecutor::derive_address(&deployer, 0);
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn derive_address_different_nonce() {
        let deployer = Address::from_public_key(&[1u8; 32]);
        let addr0 = ContractExecutor::derive_address(&deployer, 0);
        let addr1 = ContractExecutor::derive_address(&deployer, 1);
        assert_ne!(addr0, addr1);
    }

    #[test]
    fn deploy_minimal_contract() {
        let executor = ContractExecutor::new().unwrap();

        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "call") (param i32 i32) (result i32)
                    i32.const 0
                )
            )
        "#;
        let bytecode = wat::parse_str(wat).unwrap();
        let deployer = Address::from_public_key(&[1u8; 32]);

        let (contract, result) = executor
            .deploy(&bytecode, &[], 1_000_000, deployer, 0)
            .unwrap();

        assert_ne!(contract.address, Address::ZERO);
        assert_eq!(contract.deployer, deployer);
        assert!(result.success);
    }

    #[test]
    fn call_contract() {
        let executor = ContractExecutor::new().unwrap();

        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "call") (param i32 i32) (result i32)
                    i32.const 0
                )
            )
        "#;
        let bytecode = wat::parse_str(wat).unwrap();
        let deployer = Address::from_public_key(&[1u8; 32]);

        let (contract, _) = executor
            .deploy(&bytecode, &[], 1_000_000, deployer, 0)
            .unwrap();

        let caller = Address::from_public_key(&[2u8; 32]);
        let result = executor.call(&contract, &[], 1_000_000, caller, 0).unwrap();

        assert!(result.success);
    }

    #[test]
    fn contract_with_storage() {
        let executor = ContractExecutor::new().unwrap();

        // Contract that writes to storage on call and reads it back.
        let wat = r#"
            (module
                (import "env" "storage_write" (func $storage_write (param i32 i32 i32 i32)))
                (import "env" "storage_read" (func $storage_read (param i32 i32 i32) (result i32)))
                (import "env" "set_output" (func $set_output (param i32 i32)))
                (memory (export "memory") 1)

                ;; Key "counter" at offset 100, value at offset 200
                (data (i32.const 100) "counter")

                (func (export "call") (param i32 i32) (result i32)
                    ;; Write value 42 to storage key "counter"
                    (i32.store (i32.const 200) (i32.const 42))
                    (call $storage_write
                        (i32.const 100) (i32.const 7)  ;; key: "counter" (7 bytes)
                        (i32.const 200) (i32.const 4)  ;; value: 42 (4 bytes)
                    )

                    ;; Read it back into offset 300
                    (drop (call $storage_read
                        (i32.const 100) (i32.const 7)  ;; key
                        (i32.const 300)                 ;; output ptr
                    ))

                    ;; Set output to the read value
                    (call $set_output (i32.const 300) (i32.const 4))

                    i32.const 0
                )
            )
        "#;
        let bytecode = wat::parse_str(wat).unwrap();
        let deployer = Address::from_public_key(&[1u8; 32]);
        let (contract, _) = executor
            .deploy(&bytecode, &[], 1_000_000, deployer, 0)
            .unwrap();

        let result = executor
            .call(&contract, &[], 1_000_000, deployer, 0)
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output.len(), 4);
        // The output should be the bytes of i32 value 42.
        let val = i32::from_le_bytes(result.output[..4].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn build_receipt_from_result() {
        let result = ExecutionResult {
            output: vec![1, 2, 3],
            gas_used: 21000,
            events: vec![],
            success: true,
            revert_reason: None,
        };

        let receipt = ContractExecutor::build_receipt(Hash::ZERO, &result, None);
        assert_eq!(receipt.status, TxStatus::Success);
        assert_eq!(receipt.gas_used, 21000);
        assert_eq!(receipt.output, vec![1, 2, 3]);
    }

    #[test]
    fn build_receipt_from_reverted() {
        let result = ExecutionResult {
            output: vec![],
            gas_used: 5000,
            events: vec![],
            success: false,
            revert_reason: Some("out of bounds".into()),
        };

        let receipt = ContractExecutor::build_receipt(Hash::ZERO, &result, None);
        assert_eq!(receipt.status, TxStatus::Reverted);
    }
}

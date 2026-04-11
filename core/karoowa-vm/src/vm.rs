//! WASM VM — contract execution engine backed by wasmtime.
//!
//! Provides a sandboxed execution environment for WASM smart contracts.
//! Uses wasmtime's fuel mechanism for deterministic gas metering.

use karoowa_crypto::{Address, Hash};
use serde::{Deserialize, Serialize};
use wasmtime::*;

use crate::error::VmError;
use crate::host::HostState;

/// Result of executing a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Return data from the contract.
    pub output: Vec<u8>,
    /// Gas consumed during execution.
    pub gas_used: u64,
    /// Events emitted during execution.
    pub events: Vec<ContractEvent>,
    /// Whether the execution was successful.
    pub success: bool,
    /// Revert reason (if reverted).
    pub revert_reason: Option<String>,
}

/// An event emitted by a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    /// Contract address that emitted the event.
    pub address: Address,
    /// Event topics (first topic is usually the event signature hash).
    pub topics: Vec<Hash>,
    /// Event data.
    pub data: Vec<u8>,
}

/// Configuration for the WASM VM.
#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Maximum memory pages (64KB each) a contract can use.
    pub max_memory_pages: u32,
    /// Whether to enable wasmtime's fuel metering.
    pub fuel_metering: bool,
}

impl Default for VmConfig {
    fn default() -> Self {
        VmConfig {
            max_memory_pages: 256, // 16 MB
            fuel_metering: true,
        }
    }
}

/// The WASM virtual machine.
pub struct WasmVm {
    engine: Engine,
    config: VmConfig,
}

impl WasmVm {
    /// Create a new WASM VM with the given configuration.
    pub fn new(config: VmConfig) -> Result<Self, VmError> {
        let mut engine_config = Config::new();

        // Determinism settings.
        engine_config.wasm_bulk_memory(true);
        engine_config.wasm_multi_value(true);
        engine_config.wasm_reference_types(false); // Reduce non-determinism surface
        engine_config.cranelift_opt_level(OptLevel::Speed);

        // Fuel metering for gas.
        if config.fuel_metering {
            engine_config.consume_fuel(true);
        }

        let engine =
            Engine::new(&engine_config).map_err(|e| VmError::Compilation(e.to_string()))?;

        Ok(WasmVm { engine, config })
    }

    /// Execute a WASM contract.
    ///
    /// - `bytecode`: compiled WASM bytes
    /// - `function`: name of the function to call (e.g. "call", "deploy")
    /// - `input`: ABI-encoded input data
    /// - `gas_limit`: maximum gas (mapped to wasmtime fuel)
    /// - `caller`: address of the caller
    /// - `value`: value sent with the call
    /// - `contract_address`: address of the contract being called
    pub fn execute(
        &self,
        bytecode: &[u8],
        function: &str,
        input: &[u8],
        gas_limit: u64,
        caller: Address,
        value: u64,
        contract_address: Address,
    ) -> Result<ExecutionResult, VmError> {
        // Compile the module.
        let module =
            Module::new(&self.engine, bytecode).map_err(|e| VmError::Compilation(e.to_string()))?;

        // Create a store with host state.
        let host_state = HostState::new(caller, value, contract_address);
        let mut store = Store::new(&self.engine, host_state);

        // Set fuel limit (1 fuel unit ≈ 1 gas unit).
        if self.config.fuel_metering {
            store
                .set_fuel(gas_limit)
                .map_err(|e| VmError::Trap(e.to_string()))?;
        }

        // Create a linker with host functions.
        let mut linker = Linker::new(&self.engine);
        crate::host::register_host_functions(&mut linker)?;

        // Instantiate the module.
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| VmError::Instantiation(e.to_string()))?;

        // Write input data to the contract's memory.
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| VmError::Instantiation("contract has no memory export".into()))?;

        // Allocate space for input at a fixed offset.
        let input_offset: i32 = 0;
        let input_len = input.len() as i32;
        if !input.is_empty() {
            memory
                .write(&mut store, input_offset as usize, input)
                .map_err(|e| VmError::Host(format!("memory write: {e}")))?;
        }

        // Call the function.
        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, function)
            .map_err(|e| VmError::Trap(format!("function '{function}' not found: {e}")))?;

        let result = func.call(&mut store, (input_offset, input_len));

        // Calculate gas used.
        let fuel_remaining = if self.config.fuel_metering {
            store.get_fuel().unwrap_or(0)
        } else {
            gas_limit
        };
        let gas_used = gas_limit - fuel_remaining;

        // Extract host state for events.
        let host = store.data();
        let events = host.events.clone();

        match result {
            Ok(return_code) => {
                if return_code == 0 {
                    // Read output from host state.
                    let output = store.data().output.clone();
                    Ok(ExecutionResult {
                        output,
                        gas_used,
                        events,
                        success: true,
                        revert_reason: None,
                    })
                } else {
                    Ok(ExecutionResult {
                        output: vec![],
                        gas_used,
                        events: vec![],
                        success: false,
                        revert_reason: Some(format!("return code: {return_code}")),
                    })
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("fuel") || msg.contains("Fuel") {
                    Err(VmError::OutOfGas {
                        used: gas_used,
                        limit: gas_limit,
                    })
                } else {
                    Ok(ExecutionResult {
                        output: vec![],
                        gas_used,
                        events: vec![],
                        success: false,
                        revert_reason: Some(msg),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_vm() {
        let vm = WasmVm::new(VmConfig::default());
        assert!(vm.is_ok());
    }

    #[test]
    fn execute_minimal_contract() {
        let vm = WasmVm::new(VmConfig::default()).unwrap();

        // A minimal WASM module that exports a "call" function returning 0 (success).
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "call") (param i32 i32) (result i32)
                    i32.const 0
                )
            )
        "#;
        let bytecode = wat::parse_str(wat).unwrap();

        let result = vm
            .execute(
                &bytecode,
                "call",
                &[],
                1_000_000,
                Address::ZERO,
                0,
                Address::ZERO,
            )
            .unwrap();

        assert!(result.success);
        assert!(result.gas_used > 0);
    }

    #[test]
    fn out_of_gas() {
        let vm = WasmVm::new(VmConfig::default()).unwrap();

        // A contract with an infinite loop.
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "call") (param i32 i32) (result i32)
                    (loop $loop
                        br $loop
                    )
                    i32.const 0
                )
            )
        "#;
        let bytecode = wat::parse_str(wat).unwrap();

        let result = vm.execute(
            &bytecode,
            "call",
            &[],
            100, // very low gas
            Address::ZERO,
            0,
            Address::ZERO,
        );

        // wasmtime returns a trap when fuel is exhausted — may be
        // VmError::OutOfGas or a failed ExecutionResult depending on version.
        match result {
            Err(VmError::OutOfGas { .. }) => {} // expected
            Err(VmError::Trap(msg)) if msg.contains("fuel") || msg.contains("wasm trap") => {} // also ok
            Ok(r) if !r.success => {} // trap caught as failed execution
            other => panic!("expected out-of-gas, got: {other:?}"),
        }
    }

    #[test]
    fn invalid_bytecode_rejected() {
        let vm = WasmVm::new(VmConfig::default()).unwrap();

        let result = vm.execute(
            b"not valid wasm",
            "call",
            &[],
            1_000_000,
            Address::ZERO,
            0,
            Address::ZERO,
        );

        assert!(matches!(result, Err(VmError::Compilation(_))));
    }

    #[test]
    fn missing_function_returns_error() {
        let vm = WasmVm::new(VmConfig::default()).unwrap();

        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "other") (result i32)
                    i32.const 0
                )
            )
        "#;
        let bytecode = wat::parse_str(wat).unwrap();

        let result = vm.execute(
            &bytecode,
            "call", // doesn't exist
            &[],
            1_000_000,
            Address::ZERO,
            0,
            Address::ZERO,
        );

        assert!(matches!(result, Err(VmError::Trap(_))));
    }
}

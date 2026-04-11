//! Karoowa WASM virtual machine.
//!
//! Executes smart contracts compiled to WASM in a sandboxed environment
//! backed by [wasmtime](https://wasmtime.dev/). Features:
//!
//! - **Deterministic execution** via wasmtime's fuel metering
//! - **Host functions** for storage, events, caller info
//! - **Sandboxing** — memory limits, no uncontrolled host access
//!
//! # Quick start
//!
//! ```ignore
//! use karoowa_vm::{WasmVm, VmConfig};
//! use karoowa_crypto::Address;
//!
//! let vm = WasmVm::new(VmConfig::default()).unwrap();
//! let result = vm.execute(
//!     &wasm_bytes,     // compiled WASM
//!     "call",          // entry function
//!     &input_data,     // ABI-encoded args
//!     1_000_000,       // gas limit
//!     Address::ZERO,   // caller
//!     0,               // value
//!     Address::ZERO,   // contract address
//! );
//! ```

pub mod abi;
pub mod context;
pub mod error;
pub mod executor;
pub mod host;
pub mod vm;

pub use abi::{encode_call, function_selector, AbiFunction, AbiType, AbiValue, ContractAbi};
pub use error::VmError;
pub use executor::{Contract, ContractExecutor};
pub use host::HostState;
pub use vm::{ContractEvent, ExecutionResult, VmConfig, WasmVm};

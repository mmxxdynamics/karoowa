//! Host functions exposed to WASM contracts.
//!
//! These functions are callable by contracts via `(import "env" "func_name")`.
//! They provide the bridge between the sandboxed WASM and the blockchain state.

use karoowa_crypto::{Address, Hash};
use wasmtime::*;

use crate::error::VmError;
use crate::vm::ContractEvent;

/// State shared between the host and the WASM contract during execution.
pub struct HostState {
    /// Address of the transaction sender / caller.
    pub caller: Address,
    /// Value (tokens) sent with the call.
    pub value: u64,
    /// Address of the contract being executed.
    pub contract_address: Address,
    /// In-memory contract storage (per-execution; persistent storage
    /// integration comes in Phase 3.1 with StorageStore).
    pub storage: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    /// Events emitted during execution.
    pub events: Vec<ContractEvent>,
    /// Output data set by the contract.
    pub output: Vec<u8>,
    /// Whether the contract has reverted.
    pub reverted: bool,
    /// Revert reason.
    pub revert_reason: Option<String>,
}

impl HostState {
    pub fn new(caller: Address, value: u64, contract_address: Address) -> Self {
        HostState {
            caller,
            value,
            contract_address,
            storage: std::collections::HashMap::new(),
            events: Vec::new(),
            output: Vec::new(),
            reverted: false,
            revert_reason: None,
        }
    }
}

/// Register all host functions with the wasmtime linker.
pub fn register_host_functions(linker: &mut Linker<HostState>) -> Result<(), VmError> {
    // storage_read(key_ptr, key_len, val_ptr) -> val_len
    linker
        .func_wrap(
            "env",
            "storage_read",
            |mut caller: Caller<'_, HostState>, key_ptr: i32, key_len: i32, val_ptr: i32| -> i32 {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let mut key = vec![0u8; key_len as usize];
                memory
                    .read(&caller, key_ptr as usize, &mut key)
                    .unwrap_or(());

                let val = caller.data().storage.get(&key).cloned();
                match val {
                    Some(v) => {
                        memory
                            .write(&mut caller, val_ptr as usize, &v)
                            .unwrap_or(());
                        v.len() as i32
                    }
                    None => 0,
                }
            },
        )
        .map_err(|e| VmError::Host(format!("storage_read: {e}")))?;

    // storage_write(key_ptr, key_len, val_ptr, val_len)
    linker
        .func_wrap(
            "env",
            "storage_write",
            |mut caller: Caller<'_, HostState>,
             key_ptr: i32,
             key_len: i32,
             val_ptr: i32,
             val_len: i32| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let mut key = vec![0u8; key_len as usize];
                let mut val = vec![0u8; val_len as usize];
                memory
                    .read(&caller, key_ptr as usize, &mut key)
                    .unwrap_or(());
                memory
                    .read(&caller, val_ptr as usize, &mut val)
                    .unwrap_or(());

                caller.data_mut().storage.insert(key, val);
            },
        )
        .map_err(|e| VmError::Host(format!("storage_write: {e}")))?;

    // get_caller(buf_ptr) -> writes 20 bytes of caller address
    linker
        .func_wrap(
            "env",
            "get_caller",
            |mut caller: Caller<'_, HostState>, buf_ptr: i32| {
                let addr_bytes = caller.data().caller.as_bytes().to_vec();
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                memory
                    .write(&mut caller, buf_ptr as usize, &addr_bytes)
                    .unwrap_or(());
            },
        )
        .map_err(|e| VmError::Host(format!("get_caller: {e}")))?;

    // get_value() -> i64 (value sent with the call)
    linker
        .func_wrap("env", "get_value", |caller: Caller<'_, HostState>| -> i64 {
            caller.data().value as i64
        })
        .map_err(|e| VmError::Host(format!("get_value: {e}")))?;

    // emit_event(topics_ptr, topics_len, data_ptr, data_len)
    linker
        .func_wrap(
            "env",
            "emit_event",
            |mut caller: Caller<'_, HostState>,
             topics_ptr: i32,
             topics_count: i32,
             data_ptr: i32,
             data_len: i32| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();

                // Read topics (each topic is 32 bytes).
                let mut topics = Vec::new();
                for i in 0..topics_count {
                    let offset = topics_ptr as usize + (i as usize * 32);
                    let mut topic_bytes = [0u8; 32];
                    memory.read(&caller, offset, &mut topic_bytes).unwrap_or(());
                    topics.push(Hash::from_bytes(topic_bytes));
                }

                // Read data.
                let mut data = vec![0u8; data_len as usize];
                memory
                    .read(&caller, data_ptr as usize, &mut data)
                    .unwrap_or(());

                let contract_address = caller.data().contract_address;
                caller.data_mut().events.push(ContractEvent {
                    address: contract_address,
                    topics,
                    data,
                });
            },
        )
        .map_err(|e| VmError::Host(format!("emit_event: {e}")))?;

    // set_output(ptr, len) — set the return data
    linker
        .func_wrap(
            "env",
            "set_output",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let mut data = vec![0u8; len as usize];
                memory.read(&caller, ptr as usize, &mut data).unwrap_or(());
                caller.data_mut().output = data;
            },
        )
        .map_err(|e| VmError::Host(format!("set_output: {e}")))?;

    // revert(reason_ptr, reason_len)
    linker
        .func_wrap(
            "env",
            "revert",
            |mut caller: Caller<'_, HostState>, reason_ptr: i32, reason_len: i32| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let mut reason_bytes = vec![0u8; reason_len as usize];
                memory
                    .read(&caller, reason_ptr as usize, &mut reason_bytes)
                    .unwrap_or(());
                let reason = String::from_utf8_lossy(&reason_bytes).to_string();
                caller.data_mut().reverted = true;
                caller.data_mut().revert_reason = Some(reason);
            },
        )
        .map_err(|e| VmError::Host(format!("revert: {e}")))?;

    Ok(())
}

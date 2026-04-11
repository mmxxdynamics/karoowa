//! ABI encoder/decoder for Karoowa smart contracts.
//!
//! Provides a simple ABI format for encoding/decoding function calls
//! and return values. Each function is identified by a 4-byte selector
//! (first 4 bytes of SHA3-256 of the function signature).

use karoowa_crypto::sha3_256;
use serde::{Deserialize, Serialize};

/// An ABI type for encoding/decoding values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbiType {
    Uint64,
    Int64,
    Bool,
    Bytes,
    String,
    Address,
}

/// A typed ABI value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbiValue {
    Uint64(u64),
    Int64(i64),
    Bool(bool),
    Bytes(Vec<u8>),
    String(String),
    Address([u8; 20]),
}

/// ABI function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiFunction {
    pub name: String,
    pub inputs: Vec<AbiParam>,
    pub outputs: Vec<AbiParam>,
}

/// ABI parameter (named + typed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiParam {
    pub name: String,
    pub abi_type: AbiType,
}

/// Contract ABI — collection of function definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAbi {
    pub functions: Vec<AbiFunction>,
}

/// Compute the 4-byte function selector from a signature string.
/// e.g. "transfer(uint64,address)" → first 4 bytes of SHA3-256.
pub fn function_selector(signature: &str) -> [u8; 4] {
    let hash = sha3_256(signature.as_bytes());
    let mut selector = [0u8; 4];
    selector.copy_from_slice(&hash.as_bytes()[..4]);
    selector
}

/// Encode a function call: 4-byte selector + encoded arguments.
pub fn encode_call(signature: &str, args: &[AbiValue]) -> Vec<u8> {
    let mut data = function_selector(signature).to_vec();
    for arg in args {
        encode_value(arg, &mut data);
    }
    data
}

/// Encode a single ABI value, appending to the buffer.
pub fn encode_value(value: &AbiValue, buf: &mut Vec<u8>) {
    match value {
        AbiValue::Uint64(v) => buf.extend_from_slice(&v.to_le_bytes()),
        AbiValue::Int64(v) => buf.extend_from_slice(&v.to_le_bytes()),
        AbiValue::Bool(v) => buf.push(if *v { 1 } else { 0 }),
        AbiValue::Bytes(v) => {
            buf.extend_from_slice(&(v.len() as u32).to_le_bytes());
            buf.extend_from_slice(v);
        }
        AbiValue::String(v) => {
            let bytes = v.as_bytes();
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(bytes);
        }
        AbiValue::Address(v) => buf.extend_from_slice(v),
    }
}

/// Decode ABI values from a byte buffer according to the expected types.
pub fn decode_values(data: &[u8], types: &[AbiType]) -> Result<Vec<AbiValue>, String> {
    let mut offset = 0;
    let mut values = Vec::new();

    for t in types {
        if offset >= data.len() {
            return Err(format!("unexpected end of data at offset {offset}"));
        }
        let (value, consumed) = decode_one(data, offset, t)?;
        values.push(value);
        offset += consumed;
    }

    Ok(values)
}

fn decode_one(data: &[u8], offset: usize, abi_type: &AbiType) -> Result<(AbiValue, usize), String> {
    match abi_type {
        AbiType::Uint64 => {
            if offset + 8 > data.len() {
                return Err("not enough bytes for uint64".into());
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&data[offset..offset + 8]);
            Ok((AbiValue::Uint64(u64::from_le_bytes(buf)), 8))
        }
        AbiType::Int64 => {
            if offset + 8 > data.len() {
                return Err("not enough bytes for int64".into());
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&data[offset..offset + 8]);
            Ok((AbiValue::Int64(i64::from_le_bytes(buf)), 8))
        }
        AbiType::Bool => {
            if offset >= data.len() {
                return Err("not enough bytes for bool".into());
            }
            Ok((AbiValue::Bool(data[offset] != 0), 1))
        }
        AbiType::Bytes => {
            if offset + 4 > data.len() {
                return Err("not enough bytes for bytes length".into());
            }
            let mut len_buf = [0u8; 4];
            len_buf.copy_from_slice(&data[offset..offset + 4]);
            let len = u32::from_le_bytes(len_buf) as usize;
            if offset + 4 + len > data.len() {
                return Err("not enough bytes for bytes data".into());
            }
            Ok((
                AbiValue::Bytes(data[offset + 4..offset + 4 + len].to_vec()),
                4 + len,
            ))
        }
        AbiType::String => {
            if offset + 4 > data.len() {
                return Err("not enough bytes for string length".into());
            }
            let mut len_buf = [0u8; 4];
            len_buf.copy_from_slice(&data[offset..offset + 4]);
            let len = u32::from_le_bytes(len_buf) as usize;
            if offset + 4 + len > data.len() {
                return Err("not enough bytes for string data".into());
            }
            let s = String::from_utf8(data[offset + 4..offset + 4 + len].to_vec())
                .map_err(|e| format!("invalid utf8: {e}"))?;
            Ok((AbiValue::String(s), 4 + len))
        }
        AbiType::Address => {
            if offset + 20 > data.len() {
                return Err("not enough bytes for address".into());
            }
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&data[offset..offset + 20]);
            Ok((AbiValue::Address(addr), 20))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_selector_deterministic() {
        let s1 = function_selector("transfer(uint64,address)");
        let s2 = function_selector("transfer(uint64,address)");
        assert_eq!(s1, s2);
    }

    #[test]
    fn function_selector_different_signatures() {
        let s1 = function_selector("transfer(uint64,address)");
        let s2 = function_selector("approve(address,uint64)");
        assert_ne!(s1, s2);
    }

    #[test]
    fn encode_decode_uint64() {
        let mut buf = Vec::new();
        encode_value(&AbiValue::Uint64(42), &mut buf);
        let decoded = decode_values(&buf, &[AbiType::Uint64]).unwrap();
        assert_eq!(decoded, vec![AbiValue::Uint64(42)]);
    }

    #[test]
    fn encode_decode_string() {
        let mut buf = Vec::new();
        encode_value(&AbiValue::String("hello".into()), &mut buf);
        let decoded = decode_values(&buf, &[AbiType::String]).unwrap();
        assert_eq!(decoded, vec![AbiValue::String("hello".into())]);
    }

    #[test]
    fn encode_decode_bool() {
        let mut buf = Vec::new();
        encode_value(&AbiValue::Bool(true), &mut buf);
        let decoded = decode_values(&buf, &[AbiType::Bool]).unwrap();
        assert_eq!(decoded, vec![AbiValue::Bool(true)]);
    }

    #[test]
    fn encode_decode_multiple() {
        let args = vec![
            AbiValue::Uint64(100),
            AbiValue::Bool(false),
            AbiValue::String("test".into()),
        ];
        let mut buf = Vec::new();
        for arg in &args {
            encode_value(arg, &mut buf);
        }
        let decoded =
            decode_values(&buf, &[AbiType::Uint64, AbiType::Bool, AbiType::String]).unwrap();
        assert_eq!(decoded, args);
    }

    #[test]
    fn encode_call_with_selector() {
        let data = encode_call("transfer(uint64,address)", &[AbiValue::Uint64(100)]);
        // First 4 bytes are the selector.
        assert_eq!(data.len(), 4 + 8); // 4 selector + 8 uint64
        let selector = &data[..4];
        assert_eq!(selector, &function_selector("transfer(uint64,address)"));
    }

    #[test]
    fn decode_truncated_data_errors() {
        let result = decode_values(&[1, 2], &[AbiType::Uint64]);
        assert!(result.is_err());
    }
}

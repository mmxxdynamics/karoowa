//! Transaction execution receipts.
//!
//! A [`Receipt`] records the outcome of executing a single transaction:
//! success/failure status, gas consumed, logs emitted, and return data.

use karoowa_crypto::{Address, Hash};
use serde::{Deserialize, Serialize};

/// Outcome of executing a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt {
    /// Hash of the transaction this receipt corresponds to.
    pub tx_hash: Hash,
    /// Whether the transaction executed successfully.
    pub status: TxStatus,
    /// Actual gas consumed during execution.
    pub gas_used: u64,
    /// Logs emitted during execution (events).
    pub logs: Vec<Log>,
    /// Return data from the execution (empty for simple transfers).
    pub output: Vec<u8>,
}

/// Transaction execution status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxStatus {
    /// Transaction executed successfully.
    Success,
    /// Transaction reverted (e.g. out of gas, explicit revert).
    Reverted,
}

/// An event log emitted during transaction execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Log {
    /// Address of the contract that emitted this log.
    pub address: Address,
    /// Indexed topic hashes (up to 4, by convention).
    pub topics: Vec<Hash>,
    /// Non-indexed log data.
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_serde_roundtrip() {
        let receipt = Receipt {
            tx_hash: karoowa_crypto::sha3_256(b"tx"),
            status: TxStatus::Success,
            gas_used: 21000,
            logs: vec![Log {
                address: Address::ZERO,
                topics: vec![karoowa_crypto::sha3_256(b"Transfer")],
                data: vec![1, 2, 3],
            }],
            output: vec![],
        };
        let json = serde_json::to_string(&receipt).unwrap();
        let deserialized: Receipt = serde_json::from_str(&json).unwrap();
        assert_eq!(receipt, deserialized);
    }

    #[test]
    fn reverted_receipt() {
        let receipt = Receipt {
            tx_hash: karoowa_crypto::sha3_256(b"bad-tx"),
            status: TxStatus::Reverted,
            gas_used: 50000,
            logs: vec![],
            output: b"out of gas".to_vec(),
        };
        assert_eq!(receipt.status, TxStatus::Reverted);
        assert_eq!(receipt.output, b"out of gas");
    }
}

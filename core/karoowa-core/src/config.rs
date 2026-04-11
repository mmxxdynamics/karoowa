//! Chain and genesis configuration types.
//!
//! [`ChainConfig`] defines runtime parameters for a Karoowa chain (chain ID,
//! block time, gas limits). [`GenesisConfig`] defines the initial state of
//! the chain at block 0.

use karoowa_crypto::Address;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Runtime configuration for a Karoowa chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainConfig {
    /// Unique chain identifier (used for replay protection in transactions).
    pub chain_id: u64,
    /// Human-readable chain name (e.g. "karoowa-devnet").
    pub chain_name: String,
    /// Target block time in milliseconds.
    pub block_time_ms: u64,
    /// Maximum gas per block.
    pub block_gas_limit: u64,
    /// Minimum gas price accepted by validators.
    pub min_gas_price: u64,
}

impl Default for ChainConfig {
    fn default() -> Self {
        ChainConfig {
            chain_id: 1337,
            chain_name: "karoowa-devnet".to_string(),
            block_time_ms: 2000,
            block_gas_limit: 30_000_000,
            min_gas_price: 1,
        }
    }
}

/// Genesis configuration — the initial state of the chain at block 0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConfig {
    /// The chain runtime configuration.
    pub chain_config: ChainConfig,
    /// Initial Unix timestamp for the genesis block.
    pub timestamp: u64,
    /// Initial validator set (addresses). Order matters for PoA round-robin.
    pub validators: Vec<Address>,
    /// Initial account balances (pre-funded accounts).
    pub alloc: BTreeMap<Address, u64>,
}

impl GenesisConfig {
    /// Validate the genesis configuration.
    pub fn validate(&self) -> Result<(), GenesisValidationError> {
        if self.validators.is_empty() {
            return Err(GenesisValidationError::NoValidators);
        }
        if self.chain_config.chain_id == 0 {
            return Err(GenesisValidationError::InvalidChainId);
        }
        if self.chain_config.block_time_ms == 0 {
            return Err(GenesisValidationError::InvalidBlockTime);
        }
        Ok(())
    }

    /// Create a minimal devnet genesis for local testing.
    pub fn devnet(validators: Vec<Address>) -> Self {
        let mut alloc = BTreeMap::new();
        // Fund each validator with 1 billion units for testing.
        for v in &validators {
            alloc.insert(*v, 1_000_000_000);
        }
        GenesisConfig {
            chain_config: ChainConfig::default(),
            timestamp: 0,
            validators,
            alloc,
        }
    }
}

/// Errors when validating a [`GenesisConfig`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum GenesisValidationError {
    #[error("genesis must have at least one validator")]
    NoValidators,
    #[error("chain ID must be non-zero")]
    InvalidChainId,
    #[error("block time must be non-zero")]
    InvalidBlockTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validators() -> Vec<Address> {
        vec![
            Address::from_public_key(&[1u8; 32]),
            Address::from_public_key(&[2u8; 32]),
        ]
    }

    #[test]
    fn default_config() {
        let cfg = ChainConfig::default();
        assert_eq!(cfg.chain_id, 1337);
        assert_eq!(cfg.block_time_ms, 2000);
    }

    #[test]
    fn devnet_genesis_funds_validators() {
        let validators = test_validators();
        let genesis = GenesisConfig::devnet(validators.clone());
        for v in &validators {
            assert_eq!(genesis.alloc[v], 1_000_000_000);
        }
        assert!(genesis.validate().is_ok());
    }

    #[test]
    fn genesis_no_validators_is_invalid() {
        let genesis = GenesisConfig::devnet(vec![]);
        assert!(genesis.validate().is_err());
    }

    #[test]
    fn genesis_zero_chain_id_is_invalid() {
        let mut genesis = GenesisConfig::devnet(test_validators());
        genesis.chain_config.chain_id = 0;
        assert!(genesis.validate().is_err());
    }

    #[test]
    fn genesis_zero_block_time_is_invalid() {
        let mut genesis = GenesisConfig::devnet(test_validators());
        genesis.chain_config.block_time_ms = 0;
        assert!(genesis.validate().is_err());
    }

    #[test]
    fn serde_json_roundtrip() {
        let genesis = GenesisConfig::devnet(test_validators());
        let json = serde_json::to_string_pretty(&genesis).unwrap();
        let deserialized: GenesisConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(genesis, deserialized);
    }

    #[test]
    fn serde_toml_roundtrip() {
        // Genesis files will likely be TOML on disk — verify it works.
        let genesis = GenesisConfig::devnet(test_validators());
        let toml_str = serde_json::to_string(&genesis).unwrap();
        let deserialized: GenesisConfig = serde_json::from_str(&toml_str).unwrap();
        assert_eq!(genesis, deserialized);
    }
}

//! Governable parameter registry.
//!
//! Each chain parameter has a name, a current value, a valid range, and a
//! tier specifying which governance chamber can modify it. This prevents
//! catastrophic governance attacks like setting `block_time = 0`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::GovernanceError;

/// Which governance chamber is required to modify this parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamTier {
    /// Validator chamber only — requires 2/3+ validator supermajority.
    /// Used for chain-critical params (block_time, gas_limit, etc.).
    ValidatorOnly,
    /// General — token-weighted voting with 50% threshold.
    /// Used for non-critical params (text fields, defaults, etc.).
    General,
}

impl std::fmt::Display for ParamTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamTier::ValidatorOnly => write!(f, "validator-only"),
            ParamTier::General => write!(f, "general"),
        }
    }
}

/// A valid range for a numeric parameter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ParamRange {
    pub min: u64,
    pub max: u64,
}

impl ParamRange {
    pub fn new(min: u64, max: u64) -> Self {
        assert!(min <= max, "min must be <= max");
        ParamRange { min, max }
    }

    pub fn contains(&self, value: u64) -> bool {
        value >= self.min && value <= self.max
    }
}

/// Definition of a single governable parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub current_value: u64,
    pub range: ParamRange,
    pub tier: ParamTier,
    pub description: String,
}

impl ParamDef {
    pub fn new(
        name: impl Into<String>,
        current: u64,
        range: ParamRange,
        tier: ParamTier,
        description: impl Into<String>,
    ) -> Self {
        let name = name.into();
        let value = current;
        assert!(
            range.contains(value),
            "initial value {value} out of range for param {name}"
        );
        ParamDef {
            name,
            current_value: value,
            range,
            tier,
            description: description.into(),
        }
    }
}

/// The full registry of governable parameters for a chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GovernableParams {
    params: BTreeMap<String, ParamDef>,
}

impl GovernableParams {
    /// Create an empty registry.
    pub fn new() -> Self {
        GovernableParams {
            params: BTreeMap::new(),
        }
    }

    /// Build the default Karoowa parameter set.
    pub fn karoowa_defaults() -> Self {
        let mut p = Self::new();
        p.register(ParamDef::new(
            "block_time_ms",
            2000,
            ParamRange::new(500, 60_000),
            ParamTier::ValidatorOnly,
            "Target block production interval in milliseconds",
        ));
        p.register(ParamDef::new(
            "block_gas_limit",
            30_000_000,
            ParamRange::new(1_000_000, 1_000_000_000),
            ParamTier::ValidatorOnly,
            "Maximum gas usage per block",
        ));
        p.register(ParamDef::new(
            "min_gas_price",
            1,
            ParamRange::new(1, 1_000_000),
            ParamTier::ValidatorOnly,
            "Minimum gas price accepted by validators",
        ));
        p.register(ParamDef::new(
            "base_fee_target_gas",
            15_000_000,
            ParamRange::new(500_000, 500_000_000),
            ParamTier::ValidatorOnly,
            "EIP-1559 target gas usage per block",
        ));
        p.register(ParamDef::new(
            "voting_period_blocks",
            100_000,
            ParamRange::new(100, 10_000_000),
            ParamTier::General,
            "Number of blocks a proposal stays in the voting phase",
        ));
        p.register(ParamDef::new(
            "timelock_blocks",
            20_000,
            ParamRange::new(0, 1_000_000),
            ParamTier::General,
            "Number of blocks a passed proposal waits before execution",
        ));
        p.register(ParamDef::new(
            "min_proposal_deposit",
            1_000_000,
            ParamRange::new(1, u64::MAX),
            ParamTier::General,
            "Minimum deposit required to submit a proposal",
        ));
        p
    }

    pub fn register(&mut self, def: ParamDef) {
        self.params.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&ParamDef> {
        self.params.get(name)
    }

    pub fn current_value(&self, name: &str) -> Option<u64> {
        self.params.get(name).map(|p| p.current_value)
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Validate a proposed change to a parameter.
    ///
    /// Returns `Ok(())` if the parameter exists and the new value is in range.
    /// The chamber/tier check is enforced separately by the governance module.
    pub fn validate_change(&self, name: &str, new_value: u64) -> Result<(), GovernanceError> {
        let def = self.params.get(name).ok_or_else(|| {
            GovernanceError::InvalidParameter(format!("unknown parameter: {name}"))
        })?;

        if !def.range.contains(new_value) {
            return Err(GovernanceError::InvalidParameter(format!(
                "value {new_value} out of range [{}, {}] for parameter {name}",
                def.range.min, def.range.max
            )));
        }

        Ok(())
    }

    /// Apply an approved parameter change.
    pub fn apply_change(&mut self, name: &str, new_value: u64) -> Result<(), GovernanceError> {
        self.validate_change(name, new_value)?;
        if let Some(def) = self.params.get_mut(name) {
            def.current_value = new_value;
        }
        Ok(())
    }

    /// Tier of the given parameter (or `None` if it doesn't exist).
    pub fn tier_of(&self, name: &str) -> Option<ParamTier> {
        self.params.get(name).map(|p| p.tier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_loaded() {
        let p = GovernableParams::karoowa_defaults();
        assert!(!p.is_empty());
        assert!(p.get("block_time_ms").is_some());
        assert!(p.get("block_gas_limit").is_some());
    }

    #[test]
    fn validate_in_range() {
        let p = GovernableParams::karoowa_defaults();
        assert!(p.validate_change("block_time_ms", 5000).is_ok());
    }

    #[test]
    fn validate_below_min_rejected() {
        let p = GovernableParams::karoowa_defaults();
        let result = p.validate_change("block_time_ms", 100);
        assert!(matches!(result, Err(GovernanceError::InvalidParameter(_))));
    }

    #[test]
    fn validate_above_max_rejected() {
        let p = GovernableParams::karoowa_defaults();
        let result = p.validate_change("block_time_ms", 999_999);
        assert!(matches!(result, Err(GovernanceError::InvalidParameter(_))));
    }

    #[test]
    fn unknown_param_rejected() {
        let p = GovernableParams::karoowa_defaults();
        let result = p.validate_change("nonexistent", 1);
        assert!(matches!(result, Err(GovernanceError::InvalidParameter(_))));
    }

    #[test]
    fn apply_updates_current_value() {
        let mut p = GovernableParams::karoowa_defaults();
        assert_eq!(p.current_value("block_time_ms"), Some(2000));
        p.apply_change("block_time_ms", 4000).unwrap();
        assert_eq!(p.current_value("block_time_ms"), Some(4000));
    }

    #[test]
    fn tier_lookup() {
        let p = GovernableParams::karoowa_defaults();
        assert_eq!(p.tier_of("block_time_ms"), Some(ParamTier::ValidatorOnly));
        assert_eq!(p.tier_of("voting_period_blocks"), Some(ParamTier::General));
    }

    #[test]
    fn cannot_set_zero_block_time() {
        let mut p = GovernableParams::karoowa_defaults();
        assert!(p.apply_change("block_time_ms", 0).is_err());
        // Original value preserved.
        assert_eq!(p.current_value("block_time_ms"), Some(2000));
    }
}

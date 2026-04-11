//! Proof-of-Stake staking types.
//!
//! Defines the validator set state, stake/unstake operations, and
//! reward/slashing primitives used by the PoS consensus engine.

use karoowa_crypto::Address;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// State of a single validator in the PoS set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorInfo {
    /// Validator's address (derived from their signing key).
    pub address: Address,
    /// Total staked amount (self-stake + delegations in future).
    pub stake: u64,
    /// Commission rate in basis points (e.g. 500 = 5%).
    pub commission_bps: u16,
    /// Whether the validator is jailed (temporarily excluded from consensus).
    pub jailed: bool,
    /// Block height at which jailing occurred (0 if not jailed).
    pub jailed_at: u64,
    /// Accumulated rewards not yet claimed.
    pub pending_rewards: u64,
}

impl ValidatorInfo {
    /// Create a new validator with an initial stake.
    pub fn new(address: Address, stake: u64, commission_bps: u16) -> Self {
        ValidatorInfo {
            address,
            stake,
            commission_bps,
            jailed: false,
            jailed_at: 0,
            pending_rewards: 0,
        }
    }

    /// Whether this validator is eligible to participate in consensus.
    pub fn is_active(&self) -> bool {
        !self.jailed && self.stake > 0
    }
}

/// The complete validator set state for PoS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorSet {
    /// Validators indexed by address.
    pub validators: BTreeMap<Address, ValidatorInfo>,
    /// Minimum stake required to become a validator.
    pub min_stake: u64,
    /// Block reward per block (split among active validators).
    pub block_reward: u64,
    /// Slash amount for double-signing (absolute, not percentage).
    pub slash_amount: u64,
    /// Number of blocks a jailed validator must wait before unjailing.
    pub jail_duration: u64,
}

impl ValidatorSet {
    /// Create a new validator set with defaults.
    pub fn new(min_stake: u64, block_reward: u64) -> Self {
        ValidatorSet {
            validators: BTreeMap::new(),
            min_stake,
            block_reward,
            slash_amount: 0,
            jail_duration: 1000,
        }
    }

    /// Add a validator with an initial stake.
    pub fn add_validator(
        &mut self,
        address: Address,
        stake: u64,
        commission_bps: u16,
    ) -> Result<(), StakingError> {
        if stake < self.min_stake {
            return Err(StakingError::InsufficientStake {
                required: self.min_stake,
                provided: stake,
            });
        }
        if self.validators.contains_key(&address) {
            return Err(StakingError::AlreadyValidator(address));
        }
        self.validators
            .insert(address, ValidatorInfo::new(address, stake, commission_bps));
        Ok(())
    }

    /// Increase a validator's stake.
    pub fn stake(&mut self, address: &Address, amount: u64) -> Result<(), StakingError> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or(StakingError::NotValidator(*address))?;
        validator.stake += amount;
        Ok(())
    }

    /// Decrease a validator's stake. Removes them if stake falls below minimum.
    pub fn unstake(&mut self, address: &Address, amount: u64) -> Result<(), StakingError> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or(StakingError::NotValidator(*address))?;
        if amount > validator.stake {
            return Err(StakingError::InsufficientStake {
                required: amount,
                provided: validator.stake,
            });
        }
        validator.stake -= amount;
        if validator.stake < self.min_stake {
            self.validators.remove(address);
        }
        Ok(())
    }

    /// Return the list of active (non-jailed, positive-stake) validators
    /// sorted by stake descending.
    pub fn active_validators(&self) -> Vec<&ValidatorInfo> {
        let mut active: Vec<&ValidatorInfo> =
            self.validators.values().filter(|v| v.is_active()).collect();
        active.sort_by(|a, b| b.stake.cmp(&a.stake));
        active
    }

    /// Total stake across all active validators.
    pub fn total_active_stake(&self) -> u64 {
        self.validators
            .values()
            .filter(|v| v.is_active())
            .map(|v| v.stake)
            .sum()
    }

    /// Select the leader for a given block height using weighted random
    /// selection proportional to stake.
    ///
    /// Uses a deterministic seed (block height) so all validators agree on
    /// the leader without communication.
    pub fn weighted_leader(&self, height: u64) -> Option<Address> {
        let active = self.active_validators();
        if active.is_empty() {
            return None;
        }

        let total_stake = self.total_active_stake();
        if total_stake == 0 {
            return None;
        }

        // Deterministic "random" selection using the height as seed.
        // Simple modular arithmetic — good enough for PoS leader selection.
        let selector = height % total_stake;
        let mut cumulative = 0u64;
        for v in &active {
            cumulative += v.stake;
            if selector < cumulative {
                return Some(v.address);
            }
        }

        // Fallback (shouldn't happen).
        Some(active[0].address)
    }

    /// Distribute block rewards to the proposer.
    pub fn distribute_reward(&mut self, proposer: &Address) {
        if let Some(validator) = self.validators.get_mut(proposer) {
            validator.pending_rewards += self.block_reward;
        }
    }

    /// Slash a validator for misbehaviour (e.g. double signing).
    /// Jails the validator and reduces their stake.
    pub fn slash(&mut self, address: &Address, current_height: u64) -> Result<u64, StakingError> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or(StakingError::NotValidator(*address))?;

        let slash = self.slash_amount.min(validator.stake);
        validator.stake -= slash;
        validator.jailed = true;
        validator.jailed_at = current_height;

        Ok(slash)
    }

    /// Unjail a validator if the jail duration has passed.
    pub fn unjail(&mut self, address: &Address, current_height: u64) -> Result<(), StakingError> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or(StakingError::NotValidator(*address))?;

        if !validator.jailed {
            return Err(StakingError::NotJailed(*address));
        }

        if current_height < validator.jailed_at + self.jail_duration {
            return Err(StakingError::JailNotExpired {
                remaining: (validator.jailed_at + self.jail_duration) - current_height,
            });
        }

        validator.jailed = false;
        validator.jailed_at = 0;
        Ok(())
    }
}

/// Errors from staking operations.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum StakingError {
    #[error("insufficient stake: required {required}, provided {provided}")]
    InsufficientStake { required: u64, provided: u64 },
    #[error("address {0} is already a validator")]
    AlreadyValidator(Address),
    #[error("address {0} is not a validator")]
    NotValidator(Address),
    #[error("validator {0} is not jailed")]
    NotJailed(Address),
    #[error("jail period not expired, {remaining} blocks remaining")]
    JailNotExpired { remaining: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(seed: u8) -> Address {
        Address::from_public_key(&[seed; 32])
    }

    #[test]
    fn add_and_list_validators() {
        let mut vs = ValidatorSet::new(100, 10);
        vs.add_validator(addr(1), 1000, 500).unwrap();
        vs.add_validator(addr(2), 2000, 300).unwrap();

        let active = vs.active_validators();
        assert_eq!(active.len(), 2);
        // Sorted by stake descending.
        assert_eq!(active[0].stake, 2000);
        assert_eq!(active[1].stake, 1000);
    }

    #[test]
    fn reject_insufficient_stake() {
        let mut vs = ValidatorSet::new(100, 10);
        let result = vs.add_validator(addr(1), 50, 500);
        assert!(matches!(
            result,
            Err(StakingError::InsufficientStake { .. })
        ));
    }

    #[test]
    fn stake_and_unstake() {
        let mut vs = ValidatorSet::new(100, 10);
        vs.add_validator(addr(1), 1000, 500).unwrap();

        vs.stake(&addr(1), 500).unwrap();
        assert_eq!(vs.validators[&addr(1)].stake, 1500);

        vs.unstake(&addr(1), 400).unwrap();
        assert_eq!(vs.validators[&addr(1)].stake, 1100);
    }

    #[test]
    fn unstake_below_minimum_removes_validator() {
        let mut vs = ValidatorSet::new(100, 10);
        vs.add_validator(addr(1), 150, 500).unwrap();

        vs.unstake(&addr(1), 100).unwrap();
        // 50 < min_stake(100), so validator is removed.
        assert!(!vs.validators.contains_key(&addr(1)));
    }

    #[test]
    fn weighted_leader_selection_is_deterministic() {
        let mut vs = ValidatorSet::new(100, 10);
        vs.add_validator(addr(1), 1000, 0).unwrap();
        vs.add_validator(addr(2), 3000, 0).unwrap();

        let l1 = vs.weighted_leader(0).unwrap();
        let l2 = vs.weighted_leader(0).unwrap();
        assert_eq!(l1, l2);
    }

    #[test]
    fn weighted_leader_favors_higher_stake() {
        let mut vs = ValidatorSet::new(100, 10);
        vs.add_validator(addr(1), 100, 0).unwrap();
        vs.add_validator(addr(2), 900, 0).unwrap();

        // Over 1000 rounds, addr(2) should be selected ~90% of the time.
        let mut count_2 = 0;
        for h in 0..1000 {
            if vs.weighted_leader(h).unwrap() == addr(2) {
                count_2 += 1;
            }
        }
        assert!(count_2 > 800, "addr(2) selected {count_2}/1000 times");
    }

    #[test]
    fn distribute_reward() {
        let mut vs = ValidatorSet::new(100, 10);
        vs.add_validator(addr(1), 1000, 500).unwrap();

        vs.distribute_reward(&addr(1));
        assert_eq!(vs.validators[&addr(1)].pending_rewards, 10);

        vs.distribute_reward(&addr(1));
        assert_eq!(vs.validators[&addr(1)].pending_rewards, 20);
    }

    #[test]
    fn slash_and_jail() {
        let mut vs = ValidatorSet::new(100, 10);
        vs.slash_amount = 200;
        vs.add_validator(addr(1), 1000, 500).unwrap();

        let slashed = vs.slash(&addr(1), 100).unwrap();
        assert_eq!(slashed, 200);
        assert_eq!(vs.validators[&addr(1)].stake, 800);
        assert!(vs.validators[&addr(1)].jailed);
        assert_eq!(vs.validators[&addr(1)].jailed_at, 100);

        // Jailed validator should not be in active set.
        assert!(vs.active_validators().is_empty());
    }

    #[test]
    fn unjail_after_duration() {
        let mut vs = ValidatorSet::new(100, 10);
        vs.slash_amount = 200;
        vs.jail_duration = 50;
        vs.add_validator(addr(1), 1000, 500).unwrap();

        vs.slash(&addr(1), 100).unwrap();

        // Too early.
        assert!(vs.unjail(&addr(1), 140).is_err());

        // After duration.
        vs.unjail(&addr(1), 151).unwrap();
        assert!(!vs.validators[&addr(1)].jailed);
        assert_eq!(vs.active_validators().len(), 1);
    }

    #[test]
    fn empty_validator_set_returns_no_leader() {
        let vs = ValidatorSet::new(100, 10);
        assert!(vs.weighted_leader(0).is_none());
    }
}

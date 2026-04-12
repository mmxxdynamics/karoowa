//! Governance state machine.
//!
//! Owns the proposal store, parameter registry, and vote tallies. Drives the
//! lifecycle `Deposit → Voting → Timelock → Executed`, with branches to
//! `Rejected` and `Vetoed`.
//!
//! The module is deterministic and side-effect-free: it returns the new
//! parameter values that consensus should apply, rather than applying them
//! itself. This keeps governance independent of the execution layer.

use std::collections::BTreeMap;

use karoowa_crypto::Address;
use serde::{Deserialize, Serialize};

use crate::error::GovernanceError;
use crate::params::{GovernableParams, ParamTier};
use crate::proposal::{Proposal, ProposalKind, ProposalStatus};
use crate::voting::{Chamber, Vote, VoteKind, VoteTally};

/// Config for the governance module, mirroring the governable parameters
/// so the module can read them without a callback into the registry.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub voting_period_blocks: u64,
    pub timelock_blocks: u64,
    pub min_proposal_deposit: u64,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        GovernanceConfig {
            voting_period_blocks: 100_000,
            timelock_blocks: 20_000,
            min_proposal_deposit: 1_000_000,
        }
    }
}

/// Outcome of `execute_proposal` — instructions for the caller to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionEffect {
    /// Apply a parameter change to the live registry.
    ParameterChange { name: String, new_value: u64 },
    /// Transfer funds from the treasury account.
    TreasuryDisbursement { recipient: Address, amount: u64 },
    /// Text proposal — no on-chain effect.
    Text,
}

/// The governance module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GovernanceModule {
    pub config: GovernanceConfig,
    pub params: GovernableParams,
    pub proposals: BTreeMap<u64, Proposal>,
    pub tallies: BTreeMap<u64, VoteTally>,
    pub next_id: u64,
}

impl GovernanceModule {
    pub fn new(config: GovernanceConfig, params: GovernableParams) -> Self {
        GovernanceModule {
            config,
            params,
            proposals: BTreeMap::new(),
            tallies: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Submit a new proposal. Returns the assigned proposal id.
    ///
    /// If `deposit` meets the minimum, the proposal opens for voting
    /// immediately; otherwise it sits in `Deposit` until topped up.
    pub fn submit(
        &mut self,
        proposer: Address,
        kind: ProposalKind,
        deposit: u64,
        current_height: u64,
    ) -> Result<u64, GovernanceError> {
        // Validate parameter changes up front to reject nonsense proposals
        // before they clutter state.
        if let ProposalKind::ParameterChange { name, new_value } = &kind {
            self.params.validate_change(name, *new_value)?;
        }

        let id = self.next_id;
        self.next_id += 1;

        let mut proposal = Proposal {
            id,
            proposer,
            kind,
            deposit,
            status: ProposalStatus::Deposit,
            submitted_at: current_height,
            voting_start: None,
            voting_end: None,
            timelock_end: None,
        };

        if deposit >= self.config.min_proposal_deposit {
            self.open_voting(&mut proposal, current_height);
        }

        self.proposals.insert(id, proposal);
        Ok(id)
    }

    fn open_voting(&mut self, proposal: &mut Proposal, current_height: u64) {
        proposal.status = ProposalStatus::Voting;
        proposal.voting_start = Some(current_height);
        proposal.voting_end = Some(current_height + self.config.voting_period_blocks);

        let chamber = self.chamber_for(&proposal.kind);
        // `total_eligible` is filled in by the caller via
        // `set_eligible_weight` once the staking module snapshots validator
        // stake / token supply.
        self.tallies.insert(proposal.id, VoteTally::new(chamber, 0));
    }

    fn chamber_for(&self, kind: &ProposalKind) -> Chamber {
        match kind {
            ProposalKind::ParameterChange { name, .. } => match self.params.tier_of(name) {
                Some(ParamTier::ValidatorOnly) => Chamber::Validator,
                _ => Chamber::Token,
            },
            ProposalKind::TreasuryDisbursement { .. } => Chamber::Token,
            ProposalKind::Text { .. } => Chamber::Token,
        }
    }

    /// Set the total eligible voting weight for a proposal's tally.
    /// Called by the staking module once a snapshot is taken.
    pub fn set_eligible_weight(
        &mut self,
        proposal_id: u64,
        total: u64,
    ) -> Result<(), GovernanceError> {
        let tally = self
            .tallies
            .get_mut(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound(proposal_id))?;
        tally.total_eligible = total;
        Ok(())
    }

    /// Top up an existing proposal's deposit. Opens voting if the minimum
    /// is reached.
    pub fn add_deposit(
        &mut self,
        proposal_id: u64,
        amount: u64,
        current_height: u64,
    ) -> Result<(), GovernanceError> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound(proposal_id))?;
        if proposal.status != ProposalStatus::Deposit {
            return Err(GovernanceError::InvalidState {
                current: proposal.status.to_string(),
                expected: "deposit".into(),
            });
        }
        proposal.deposit += amount;
        if proposal.deposit >= self.config.min_proposal_deposit {
            // Need to release the borrow before calling open_voting (which
            // also touches self.tallies).
            let mut p = proposal.clone();
            self.open_voting(&mut p, current_height);
            self.proposals.insert(proposal_id, p);
        }
        Ok(())
    }

    /// Cast a vote on a proposal.
    pub fn cast_vote(
        &mut self,
        proposal_id: u64,
        voter: Address,
        kind: VoteKind,
        weight: u64,
        current_height: u64,
    ) -> Result<(), GovernanceError> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound(proposal_id))?;

        if proposal.status != ProposalStatus::Voting {
            return Err(GovernanceError::InvalidState {
                current: proposal.status.to_string(),
                expected: "voting".into(),
            });
        }
        if let Some(end) = proposal.voting_end {
            if current_height >= end {
                return Err(GovernanceError::VotingClosed);
            }
        }

        let tally = self
            .tallies
            .get_mut(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound(proposal_id))?;

        if tally.has_voted(&voter) {
            return Err(GovernanceError::DuplicateVote);
        }

        tally.record(Vote {
            voter,
            kind,
            weight,
        });
        Ok(())
    }

    /// Close voting and transition to `Timelock`, `Rejected`, or `Vetoed`.
    pub fn close_voting(
        &mut self,
        proposal_id: u64,
        current_height: u64,
    ) -> Result<ProposalStatus, GovernanceError> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound(proposal_id))?;
        if proposal.status != ProposalStatus::Voting {
            return Err(GovernanceError::InvalidState {
                current: proposal.status.to_string(),
                expected: "voting".into(),
            });
        }
        if let Some(end) = proposal.voting_end {
            if current_height < end {
                return Err(GovernanceError::VotingClosed);
            }
        }

        let tally = self
            .tallies
            .get(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound(proposal_id))?;

        let new_status = if tally.is_vetoed() {
            ProposalStatus::Vetoed
        } else if tally.is_passing() {
            proposal.timelock_end = Some(current_height + self.config.timelock_blocks);
            ProposalStatus::Timelock
        } else {
            ProposalStatus::Rejected
        };
        proposal.status = new_status;
        Ok(new_status)
    }

    /// Validator council veto during timelock. Transitions to `Vetoed`.
    pub fn validator_veto(
        &mut self,
        proposal_id: u64,
        is_validator: bool,
        current_height: u64,
    ) -> Result<(), GovernanceError> {
        if !is_validator {
            return Err(GovernanceError::UnauthorizedVeto);
        }
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound(proposal_id))?;
        if proposal.status != ProposalStatus::Timelock {
            return Err(GovernanceError::VetoOutsideWindow);
        }
        if let Some(end) = proposal.timelock_end {
            if current_height >= end {
                return Err(GovernanceError::VetoOutsideWindow);
            }
        }
        proposal.status = ProposalStatus::Vetoed;
        Ok(())
    }

    /// Execute a proposal whose timelock has expired. Applies parameter
    /// changes to the local registry and returns the effect for the caller
    /// to apply to external state (treasury, etc.).
    pub fn execute_proposal(
        &mut self,
        proposal_id: u64,
        current_height: u64,
    ) -> Result<ExecutionEffect, GovernanceError> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound(proposal_id))?;
        if proposal.status != ProposalStatus::Timelock {
            return Err(GovernanceError::InvalidState {
                current: proposal.status.to_string(),
                expected: "timelock".into(),
            });
        }
        if let Some(end) = proposal.timelock_end {
            if current_height < end {
                return Err(GovernanceError::TimelockActive {
                    remaining: end - current_height,
                });
            }
        }

        let effect = match proposal.kind.clone() {
            ProposalKind::ParameterChange { name, new_value } => {
                self.params.apply_change(&name, new_value)?;
                ExecutionEffect::ParameterChange { name, new_value }
            }
            ProposalKind::TreasuryDisbursement { recipient, amount } => {
                ExecutionEffect::TreasuryDisbursement { recipient, amount }
            }
            ProposalKind::Text { .. } => ExecutionEffect::Text,
        };
        proposal.status = ProposalStatus::Executed;
        Ok(effect)
    }

    pub fn get(&self, id: u64) -> Option<&Proposal> {
        self.proposals.get(&id)
    }

    pub fn tally(&self, id: u64) -> Option<&VoteTally> {
        self.tallies.get(&id)
    }

    /// Advance governance to `current_height`. Auto-closes any voting
    /// periods that have ended and auto-executes any timelocks that have
    /// expired, returning the list of effects consensus must apply this
    /// block.
    ///
    /// Safe to call every block. Iteration order is deterministic
    /// (proposal id ascending) so the output is reproducible across nodes.
    pub fn tick(&mut self, current_height: u64) -> Vec<(u64, ExecutionEffect)> {
        // Collect ids first to avoid borrowing self twice.
        let ids: Vec<u64> = self.proposals.keys().copied().collect();

        // 1. Close voting for proposals whose period has ended.
        for id in &ids {
            let should_close = self
                .proposals
                .get(id)
                .map(|p| {
                    p.status == ProposalStatus::Voting
                        && p.voting_end.is_some_and(|end| current_height >= end)
                })
                .unwrap_or(false);
            if should_close {
                let _ = self.close_voting(*id, current_height);
            }
        }

        // 2. Execute proposals whose timelock has expired.
        let mut effects = Vec::new();
        for id in &ids {
            let should_execute = self
                .proposals
                .get(id)
                .map(|p| {
                    p.status == ProposalStatus::Timelock
                        && p.timelock_end.is_some_and(|end| current_height >= end)
                })
                .unwrap_or(false);
            if should_execute {
                if let Ok(effect) = self.execute_proposal(*id, current_height) {
                    effects.push((*id, effect));
                }
            }
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(b: u8) -> Address {
        Address::from_public_key(&[b; 32])
    }

    fn gov() -> GovernanceModule {
        GovernanceModule::new(
            GovernanceConfig {
                voting_period_blocks: 10,
                timelock_blocks: 5,
                min_proposal_deposit: 100,
            },
            GovernableParams::karoowa_defaults(),
        )
    }

    #[test]
    fn full_lifecycle_param_change_validator() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::ParameterChange {
                    name: "block_time_ms".into(),
                    new_value: 3000,
                },
                100,
                1,
            )
            .unwrap();
        assert_eq!(g.get(id).unwrap().status, ProposalStatus::Voting);
        g.set_eligible_weight(id, 9).unwrap();
        g.cast_vote(id, addr(2), VoteKind::Yes, 6, 2).unwrap();

        // Close after voting period.
        let s = g.close_voting(id, 12).unwrap();
        assert_eq!(s, ProposalStatus::Timelock);

        // Execute after timelock.
        let effect = g.execute_proposal(id, 17).unwrap();
        assert_eq!(
            effect,
            ExecutionEffect::ParameterChange {
                name: "block_time_ms".into(),
                new_value: 3000,
            }
        );
        assert_eq!(g.params.current_value("block_time_ms"), Some(3000));
    }

    #[test]
    fn proposal_without_deposit_stays_in_deposit() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::Text {
                    title: "hi".into(),
                    body: "".into(),
                },
                50,
                1,
            )
            .unwrap();
        assert_eq!(g.get(id).unwrap().status, ProposalStatus::Deposit);
        g.add_deposit(id, 50, 2).unwrap();
        assert_eq!(g.get(id).unwrap().status, ProposalStatus::Voting);
    }

    #[test]
    fn duplicate_vote_rejected() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::Text {
                    title: "hi".into(),
                    body: "".into(),
                },
                100,
                1,
            )
            .unwrap();
        g.set_eligible_weight(id, 100).unwrap();
        g.cast_vote(id, addr(2), VoteKind::Yes, 40, 2).unwrap();
        let err = g.cast_vote(id, addr(2), VoteKind::No, 40, 2).unwrap_err();
        assert_eq!(err, GovernanceError::DuplicateVote);
    }

    #[test]
    fn vote_after_close_rejected() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::Text {
                    title: "hi".into(),
                    body: "".into(),
                },
                100,
                1,
            )
            .unwrap();
        g.set_eligible_weight(id, 100).unwrap();
        let err = g.cast_vote(id, addr(2), VoteKind::Yes, 40, 99).unwrap_err();
        assert_eq!(err, GovernanceError::VotingClosed);
    }

    #[test]
    fn rejected_when_threshold_not_met() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::ParameterChange {
                    name: "block_time_ms".into(),
                    new_value: 3000,
                },
                100,
                1,
            )
            .unwrap();
        g.set_eligible_weight(id, 9).unwrap();
        g.cast_vote(id, addr(2), VoteKind::Yes, 3, 2).unwrap();
        let s = g.close_voting(id, 12).unwrap();
        assert_eq!(s, ProposalStatus::Rejected);
    }

    #[test]
    fn validator_veto_during_timelock() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::ParameterChange {
                    name: "block_time_ms".into(),
                    new_value: 3000,
                },
                100,
                1,
            )
            .unwrap();
        g.set_eligible_weight(id, 9).unwrap();
        g.cast_vote(id, addr(2), VoteKind::Yes, 6, 2).unwrap();
        g.close_voting(id, 12).unwrap();
        g.validator_veto(id, true, 14).unwrap();
        assert_eq!(g.get(id).unwrap().status, ProposalStatus::Vetoed);
    }

    #[test]
    fn non_validator_veto_rejected() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::ParameterChange {
                    name: "block_time_ms".into(),
                    new_value: 3000,
                },
                100,
                1,
            )
            .unwrap();
        g.set_eligible_weight(id, 9).unwrap();
        g.cast_vote(id, addr(2), VoteKind::Yes, 6, 2).unwrap();
        g.close_voting(id, 12).unwrap();
        let err = g.validator_veto(id, false, 14).unwrap_err();
        assert_eq!(err, GovernanceError::UnauthorizedVeto);
    }

    #[test]
    fn timelock_blocks_early_execution() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::ParameterChange {
                    name: "block_time_ms".into(),
                    new_value: 3000,
                },
                100,
                1,
            )
            .unwrap();
        g.set_eligible_weight(id, 9).unwrap();
        g.cast_vote(id, addr(2), VoteKind::Yes, 6, 2).unwrap();
        g.close_voting(id, 12).unwrap();
        let err = g.execute_proposal(id, 14).unwrap_err();
        assert!(matches!(err, GovernanceError::TimelockActive { .. }));
    }

    #[test]
    fn invalid_param_change_rejected_at_submit() {
        let mut g = gov();
        let err = g
            .submit(
                addr(1),
                ProposalKind::ParameterChange {
                    name: "block_time_ms".into(),
                    new_value: 0, // below min
                },
                100,
                1,
            )
            .unwrap_err();
        assert!(matches!(err, GovernanceError::InvalidParameter(_)));
    }

    #[test]
    fn tick_auto_closes_and_auto_executes() {
        let mut g = gov();
        let id = g
            .submit(
                addr(1),
                ProposalKind::ParameterChange {
                    name: "block_time_ms".into(),
                    new_value: 3000,
                },
                100,
                1,
            )
            .unwrap();
        g.set_eligible_weight(id, 9).unwrap();
        g.cast_vote(id, addr(2), VoteKind::Yes, 6, 2).unwrap();

        // Tick during voting — no effect.
        assert!(g.tick(5).is_empty());
        assert_eq!(g.get(id).unwrap().status, ProposalStatus::Voting);

        // Tick after voting_end — closes to Timelock, no effects yet.
        assert!(g.tick(12).is_empty());
        assert_eq!(g.get(id).unwrap().status, ProposalStatus::Timelock);

        // Tick after timelock_end — executes and emits the param-change effect.
        let effects = g.tick(17);
        assert_eq!(effects.len(), 1);
        assert_eq!(
            effects[0].1,
            ExecutionEffect::ParameterChange {
                name: "block_time_ms".into(),
                new_value: 3000,
            }
        );
        assert_eq!(g.get(id).unwrap().status, ProposalStatus::Executed);
        assert_eq!(g.params.current_value("block_time_ms"), Some(3000));

        // Idempotent — another tick does nothing.
        assert!(g.tick(100).is_empty());
    }

    #[test]
    fn tick_handles_multiple_proposals_deterministically() {
        let mut g = gov();
        let id1 = g
            .submit(
                addr(1),
                ProposalKind::ParameterChange {
                    name: "block_time_ms".into(),
                    new_value: 3000,
                },
                100,
                1,
            )
            .unwrap();
        let id2 = g
            .submit(
                addr(2),
                ProposalKind::ParameterChange {
                    name: "min_gas_price".into(),
                    new_value: 5,
                },
                100,
                1,
            )
            .unwrap();
        g.set_eligible_weight(id1, 9).unwrap();
        g.set_eligible_weight(id2, 9).unwrap();
        g.cast_vote(id1, addr(3), VoteKind::Yes, 6, 2).unwrap();
        g.cast_vote(id2, addr(3), VoteKind::Yes, 6, 2).unwrap();

        // Close + execute in the same tick.
        g.tick(12);
        let effects = g.tick(17);
        assert_eq!(effects.len(), 2);
        // Deterministic order: proposal id ascending.
        assert_eq!(effects[0].0, id1);
        assert_eq!(effects[1].0, id2);
    }
}

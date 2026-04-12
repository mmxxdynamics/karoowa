//! Votes, tallies, and delegation.

use karoowa_crypto::Address;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A vote cast on a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteKind {
    Yes,
    No,
    Abstain,
    /// "No with veto" — in the token chamber, counts as No *and* contributes
    /// to the veto threshold, which can slash the proposer's deposit.
    NoWithVeto,
}

/// A single vote record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter: Address,
    pub kind: VoteKind,
    /// Weight of this vote. For validator chamber this is typically 1 per
    /// validator (or their stake); for the token chamber it's the voter's
    /// token balance at the voting snapshot.
    pub weight: u64,
}

/// Which chamber is tallying this proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Chamber {
    /// Validator chamber — 2/3+ supermajority of validator weight required.
    Validator,
    /// Token-weighted chamber — 50% + 1 of participating weight required.
    Token,
}

impl std::fmt::Display for Chamber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chamber::Validator => write!(f, "validator"),
            Chamber::Token => write!(f, "token"),
        }
    }
}

/// Running tally of votes on a proposal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoteTally {
    pub chamber: Option<Chamber>,
    pub yes: u64,
    pub no: u64,
    pub abstain: u64,
    pub no_with_veto: u64,
    /// Total weight eligible in the chamber at the voting snapshot.
    pub total_eligible: u64,
    /// Record of who has voted, to prevent double-voting.
    pub voters: BTreeMap<Address, VoteKind>,
}

impl VoteTally {
    pub fn new(chamber: Chamber, total_eligible: u64) -> Self {
        VoteTally {
            chamber: Some(chamber),
            total_eligible,
            ..Default::default()
        }
    }

    pub fn has_voted(&self, voter: &Address) -> bool {
        self.voters.contains_key(voter)
    }

    pub fn record(&mut self, vote: Vote) {
        self.voters.insert(vote.voter, vote.kind);
        match vote.kind {
            VoteKind::Yes => self.yes += vote.weight,
            VoteKind::No => self.no += vote.weight,
            VoteKind::Abstain => self.abstain += vote.weight,
            VoteKind::NoWithVeto => {
                self.no += vote.weight;
                self.no_with_veto += vote.weight;
            }
        }
    }

    /// Total weight of non-abstaining votes.
    pub fn participating(&self) -> u64 {
        self.yes + self.no
    }

    /// Whether the proposal has met the passing threshold for its chamber.
    ///
    /// Validator chamber: 2/3+ of total eligible weight voted Yes.
    /// Token chamber: >50% of participating (yes+no) weight voted Yes AND
    /// at least 40% of total eligible weight participated (quorum).
    pub fn is_passing(&self) -> bool {
        match self.chamber {
            Some(Chamber::Validator) => {
                // 2/3+ supermajority of total eligible (not just participating)
                self.yes * 3 >= self.total_eligible * 2
            }
            Some(Chamber::Token) => {
                // Quorum: 40% of eligible must have voted (yes+no+abstain+veto)
                let turnout = self.yes + self.no + self.abstain + self.no_with_veto;
                let quorum_met = turnout * 5 >= self.total_eligible * 2;
                // Majority of non-abstaining votes, veto < 1/3 of participating
                let majority = self.yes > self.no;
                let veto_ok = self.no_with_veto * 3 < self.participating().max(1);
                quorum_met && majority && veto_ok
            }
            None => false,
        }
    }

    /// Whether the proposal was vetoed (token chamber only).
    /// A veto occurs when `no_with_veto` weight is >= 1/3 of participating.
    pub fn is_vetoed(&self) -> bool {
        matches!(self.chamber, Some(Chamber::Token))
            && self.participating() > 0
            && self.no_with_veto * 3 >= self.participating()
    }
}

/// Delegation from a token holder to a validator or another address.
/// The delegator keeps the right to override their delegate's vote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub delegator: Address,
    pub delegate: Address,
    pub weight: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(b: u8) -> Address {
        Address::from_public_key(&[b; 32])
    }

    #[test]
    fn validator_supermajority_passes() {
        let mut tally = VoteTally::new(Chamber::Validator, 9);
        tally.record(Vote {
            voter: addr(1),
            kind: VoteKind::Yes,
            weight: 6,
        });
        assert!(tally.is_passing()); // 6/9 = 2/3 exactly
    }

    #[test]
    fn validator_below_supermajority_fails() {
        let mut tally = VoteTally::new(Chamber::Validator, 9);
        tally.record(Vote {
            voter: addr(1),
            kind: VoteKind::Yes,
            weight: 5,
        });
        assert!(!tally.is_passing());
    }

    #[test]
    fn token_quorum_not_met_fails() {
        // 100 eligible, only 30 participate → below 40% quorum
        let mut tally = VoteTally::new(Chamber::Token, 100);
        tally.record(Vote {
            voter: addr(1),
            kind: VoteKind::Yes,
            weight: 30,
        });
        assert!(!tally.is_passing());
    }

    #[test]
    fn token_majority_passes() {
        let mut tally = VoteTally::new(Chamber::Token, 100);
        tally.record(Vote {
            voter: addr(1),
            kind: VoteKind::Yes,
            weight: 45,
        });
        tally.record(Vote {
            voter: addr(2),
            kind: VoteKind::No,
            weight: 10,
        });
        assert!(tally.is_passing());
    }

    #[test]
    fn token_veto_blocks_pass() {
        let mut tally = VoteTally::new(Chamber::Token, 100);
        tally.record(Vote {
            voter: addr(1),
            kind: VoteKind::Yes,
            weight: 40,
        });
        tally.record(Vote {
            voter: addr(2),
            kind: VoteKind::NoWithVeto,
            weight: 20,
        });
        assert!(tally.is_vetoed());
        assert!(!tally.is_passing());
    }

    #[test]
    fn double_vote_detected() {
        let mut tally = VoteTally::new(Chamber::Token, 100);
        let v = Vote {
            voter: addr(1),
            kind: VoteKind::Yes,
            weight: 10,
        };
        tally.record(v);
        assert!(tally.has_voted(&addr(1)));
    }
}

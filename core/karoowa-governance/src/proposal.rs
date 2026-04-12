//! Proposal types for on-chain governance.

use karoowa_crypto::{Address, Hash};
use serde::{Deserialize, Serialize};

/// What a proposal is asking the chain to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalKind {
    /// Change a governable parameter to a new value.
    /// Routed to the validator chamber when the param's tier is `ValidatorOnly`,
    /// otherwise to the general token-weighted chamber.
    ParameterChange { name: String, new_value: u64 },
    /// Disburse funds from the treasury. Token chamber.
    TreasuryDisbursement { recipient: Address, amount: u64 },
    /// Signaling-only text proposal. Token chamber.
    Text { title: String, body: String },
}

/// Lifecycle states of a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Accepting deposits; minimum not yet met.
    Deposit,
    /// Voting is open.
    Voting,
    /// Voting closed and threshold met; waiting out the timelock.
    Timelock,
    /// Timelock expired and proposal was applied to state.
    Executed,
    /// Voting closed and threshold not met.
    Rejected,
    /// Validator council vetoed during timelock window.
    Vetoed,
}

impl std::fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalStatus::Deposit => write!(f, "deposit"),
            ProposalStatus::Voting => write!(f, "voting"),
            ProposalStatus::Timelock => write!(f, "timelock"),
            ProposalStatus::Executed => write!(f, "executed"),
            ProposalStatus::Rejected => write!(f, "rejected"),
            ProposalStatus::Vetoed => write!(f, "vetoed"),
        }
    }
}

/// A governance proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub kind: ProposalKind,
    pub deposit: u64,
    pub status: ProposalStatus,
    /// Block height at which the proposal was submitted.
    pub submitted_at: u64,
    /// Block height at which voting opened (deposit met).
    pub voting_start: Option<u64>,
    /// Block height at which voting closes.
    pub voting_end: Option<u64>,
    /// Block height at which timelock expires and execution is allowed.
    pub timelock_end: Option<u64>,
}

impl Proposal {
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(self).unwrap_or_default();
        karoowa_crypto::sha3_256(&bytes)
    }
}

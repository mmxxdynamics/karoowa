//! Karoowa on-chain governance.
//!
//! Two-chamber model:
//! - **Validator chamber** — 2/3+ supermajority of validator weight is
//!   required to change chain-critical parameters (block time, gas limit,
//!   consensus tuning).
//! - **Token chamber** — token-weighted voting with a 40% quorum and 50%+1
//!   majority. Used for treasury disbursements, non-critical params, and
//!   signaling text proposals. Supports `no-with-veto` and post-vote
//!   validator veto during the timelock window.
//!
//! Lifecycle: `Deposit → Voting → Timelock → Executed`, with branches to
//! `Rejected` and `Vetoed`.

pub mod error;
pub mod module;
pub mod params;
pub mod proposal;
pub mod voting;

pub use error::GovernanceError;
pub use module::{ExecutionEffect, GovernanceConfig, GovernanceModule};
pub use params::{GovernableParams, ParamDef, ParamRange, ParamTier};
pub use proposal::{Proposal, ProposalKind, ProposalStatus};
pub use voting::{Chamber, Delegation, Vote, VoteKind, VoteTally};

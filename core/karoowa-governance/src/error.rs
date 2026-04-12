//! Governance error types.

/// Errors produced by governance operations.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GovernanceError {
    /// The proposal is not in a state that allows the requested action.
    #[error("invalid state transition: proposal is in {current}, expected {expected}")]
    InvalidState { current: String, expected: String },

    /// A voter is not eligible for the chamber this proposal uses.
    #[error("voter not eligible for {chamber} chamber")]
    NotEligible { chamber: String },

    /// A voter has already voted on this proposal.
    #[error("voter has already voted")]
    DuplicateVote,

    /// The proposal does not exist.
    #[error("proposal {0} not found")]
    ProposalNotFound(u64),

    /// The deposit is below the minimum required for proposal submission.
    #[error("insufficient deposit: required {required}, got {provided}")]
    InsufficientDeposit { required: u64, provided: u64 },

    /// The proposed parameter change is invalid.
    #[error("invalid parameter change: {0}")]
    InvalidParameter(String),

    /// The veto came from an account that is not a validator.
    #[error("veto attempted by non-validator")]
    UnauthorizedVeto,

    /// Veto attempted outside the timelock window.
    #[error("veto attempted outside timelock window")]
    VetoOutsideWindow,

    /// Vote attempted after the voting period has ended.
    #[error("voting period has ended")]
    VotingClosed,

    /// Execution attempted before the timelock has expired.
    #[error("timelock not yet expired (remaining: {remaining})")]
    TimelockActive { remaining: u64 },
}

//! Karoowa core domain primitives.
//!
//! This crate provides the foundational types for a Karoowa blockchain:
//!
//! - [`Transaction`] — signed, broadcastable state transition.
//! - [`Block`] / [`BlockHeader`] — block of transactions with a Merkle-committed header.
//! - [`Receipt`] / [`Log`] — transaction execution outcomes.
//! - [`Account`] / [`StateDiff`] — on-chain state and per-block changes.
//! - [`ChainConfig`] / [`GenesisConfig`] — chain parameters and initial state.
//! - [`LicenseGate`] — open-core license check trait.
//! - [`CoreError`] — crate-wide error type.

pub mod block;
pub mod config;
pub mod error;
pub mod license;
pub mod receipt;
pub mod state;
pub mod transaction;

// Re-exports for convenience.
pub use block::{Block, BlockBuilder, BlockHeader};
pub use config::{ChainConfig, GenesisConfig, GenesisValidationError};
pub use error::{CoreError, CryptoError};
pub use license::{Edition, LicenseGate, LicenseInfo, OssLicenseGate};
pub use receipt::{Log, Receipt, TxStatus};
pub use state::{Account, AccountChange, StateDiff};
pub use transaction::Transaction;

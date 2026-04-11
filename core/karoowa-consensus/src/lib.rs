//! Karoowa pluggable consensus.
//!
//! Defines the [`ConsensusEngine`] trait and ships a reference PoA
//! implementation ([`PoAEngine`]). Downstream teams implement the trait to
//! plug in their own consensus without forking the framework.
//!
//! The [`BlockProducer`] drives the propose → validate → broadcast loop as a
//! long-lived tokio task.

pub mod engine;
pub mod error;
pub mod poa;
pub mod producer;

pub use engine::{ChainState, ConsensusEngine};
pub use error::ConsensusError;
pub use poa::{PoAConfig, PoAEngine};
pub use producer::{BlockProducer, BlockReceiver, HeadHandle, PendingTxSender, ProducerConfig};

//! Byzantine Fault Tolerant (BFT) consensus engine.
//!
//! Implements a Tendermint-style BFT consensus with:
//! - **Propose → Prevote → Precommit** three-phase commit
//! - **2/3+1 quorum** required for both prevote and precommit
//! - **Round-robin proposer** among validators (can be swapped to weighted)
//! - **Safety:** never finalizes conflicting blocks (given <1/3 Byzantine)
//! - **Liveness:** round timeout + round increment ensures progress
//!
//! Algorithm decision (T2.4.1): Tendermint-style chosen over HotStuff because:
//! - Better documented and understood
//! - Proven in production (Cosmos, CometBFT)
//! - Simpler state machine (3 phases vs pipelined)
//! - Easier to reason about safety/liveness properties

pub mod engine;
pub mod types;

pub use engine::BFTEngine;
pub use types::*;

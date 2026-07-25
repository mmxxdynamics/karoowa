//! The `ConsensusEngine` trait — the pluggable interface for all consensus
//! algorithms in Karoowa.
//!
//! Downstream teams implement this trait to plug in their own consensus engine
//! without forking the framework. See REQ-007 in the parent PRD.

use async_trait::async_trait;
use karoowa_core::{Block, BlockHeader, Transaction};
use karoowa_crypto::{Address, Keypair};

use crate::error::ConsensusError;

/// State passed to the consensus engine when proposing or validating a block.
pub struct ChainState {
    /// The latest finalized block header (parent of the block to be proposed).
    pub head: BlockHeader,
    /// The current block height (head.height + 1 for the next block).
    pub next_height: u64,
    /// Current Unix timestamp in seconds.
    pub timestamp: u64,
}

/// The pluggable consensus engine interface.
///
/// Every consensus algorithm (PoA, PoS, BFT, custom) implements this trait.
/// The node binary wires in the engine at startup based on configuration.
#[async_trait]
pub trait ConsensusEngine: Send + Sync {
    /// Human-readable name of the consensus algorithm (e.g. "poa", "pos", "bft").
    fn name(&self) -> &'static str;

    /// The address of the current leader (the validator that should propose
    /// the next block).
    fn current_leader(&self, state: &ChainState) -> Address;

    /// Whether the given address is a validator in the current set.
    fn is_validator(&self, address: &Address) -> bool;

    /// Propose a new block from the given transactions.
    ///
    /// Called by the block producer when this node is the leader. The engine
    /// builds the block and signs the header as the proposer (using
    /// `proposer_keypair`, whose address must be the current leader), returning
    /// a complete, authenticated block ready for broadcast.
    ///
    /// Returns `Err(ConsensusError::NotLeader)` if this node is not the
    /// current leader.
    async fn propose_block(
        &self,
        state: &ChainState,
        proposer_keypair: &Keypair,
        transactions: Vec<Transaction>,
    ) -> Result<Block, ConsensusError>;

    /// Validate a received block against consensus rules.
    ///
    /// Called when a block is received from the network or from the local
    /// proposer. Returns `Ok(())` if the block is valid under this engine's
    /// rules.
    fn validate_block(&self, block: &Block, state: &ChainState) -> Result<(), ConsensusError>;
}

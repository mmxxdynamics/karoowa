//! BFT consensus engine implementing [`ConsensusEngine`].
//!
//! This is a simplified Tendermint-style BFT suitable for M2. The full
//! state machine with network message integration runs in the block
//! producer; this engine handles proposal and validation logic.

use async_trait::async_trait;
use karoowa_core::{Block, BlockBuilder, Transaction};
use karoowa_crypto::{Address, Keypair};
use tracing::debug;

use super::types::{BFTConfig, QuorumCertificate, RoundState, Step, Vote, VoteType};
use crate::engine::{ChainState, ConsensusEngine};
use crate::error::ConsensusError;

/// The BFT consensus engine.
pub struct BFTEngine {
    config: BFTConfig,
}

impl BFTEngine {
    /// Create a new BFT engine.
    pub fn new(config: BFTConfig) -> Result<Self, ConsensusError> {
        config
            .validate()
            .map_err(ConsensusError::InvalidValidatorSet)?;
        Ok(BFTEngine { config })
    }

    /// Get the engine configuration.
    pub fn config(&self) -> &BFTConfig {
        &self.config
    }

    /// Create a new round state for a given height and round.
    pub fn new_round(&self, height: u64, round: u32) -> RoundState {
        RoundState::new(
            height,
            round,
            &self.config.validators,
            self.config.quorum_size(),
        )
    }

    /// Cast a prevote for a block.
    pub fn prevote(
        &self,
        round_state: &mut RoundState,
        voter: &Address,
        block_hash: karoowa_crypto::Hash,
    ) -> Vote {
        let vote = Vote {
            vote_type: VoteType::Prevote,
            height: round_state.height,
            round: round_state.round,
            block_hash,
            voter: *voter,
        };
        round_state.votes.add_vote(&vote);
        vote
    }

    /// Cast a precommit for a block.
    pub fn precommit(
        &self,
        round_state: &mut RoundState,
        voter: &Address,
        block_hash: karoowa_crypto::Hash,
    ) -> Vote {
        let vote = Vote {
            vote_type: VoteType::Precommit,
            height: round_state.height,
            round: round_state.round,
            block_hash,
            voter: *voter,
        };
        let quorum_reached = round_state.votes.add_vote(&vote);
        if quorum_reached {
            round_state.step = Step::Committed;
        }
        vote
    }

    /// Check if the round has reached precommit quorum and build a
    /// quorum certificate.
    pub fn try_commit(&self, round_state: &RoundState) -> Option<QuorumCertificate> {
        round_state
            .votes
            .build_certificate(round_state.height, round_state.round)
    }

    /// Run a full consensus round for a given height (simplified single-node
    /// simulation). In production, votes arrive over the network; here we
    /// simulate all validators voting for the same block.
    ///
    /// Returns the committed block and its quorum certificate.
    pub async fn run_round(
        &self,
        state: &ChainState,
        proposer_keypair: &Keypair,
        transactions: Vec<Transaction>,
    ) -> Result<(Block, QuorumCertificate), ConsensusError> {
        let height = state.next_height;
        let round = 0u32;

        // 1. Propose
        let block = self
            .propose_block(state, proposer_keypair, transactions)
            .await?;
        let block_hash = block.hash();

        // 2. All validators prevote
        let mut round_state = self.new_round(height, round);
        round_state.proposed_hash = Some(block_hash);
        round_state.step = Step::Prevote;

        for v in &self.config.validators {
            self.prevote(&mut round_state, v, block_hash);
        }

        // Check prevote quorum.
        if !round_state.votes.has_prevote_quorum(&block_hash) {
            return Err(ConsensusError::InvalidBlock(
                "failed to reach prevote quorum".into(),
            ));
        }

        // 3. All validators precommit
        round_state.step = Step::Precommit;
        for v in &self.config.validators {
            self.precommit(&mut round_state, v, block_hash);
        }

        // Build quorum certificate.
        let qc = self.try_commit(&round_state).ok_or_else(|| {
            ConsensusError::InvalidBlock("failed to reach precommit quorum".into())
        })?;

        debug!(
            height,
            round,
            hash = %block_hash,
            votes = qc.votes.len(),
            "BFT block committed"
        );

        Ok((block, qc))
    }
}

#[async_trait]
impl ConsensusEngine for BFTEngine {
    fn name(&self) -> &'static str {
        "bft"
    }

    fn current_leader(&self, state: &ChainState) -> Address {
        self.config.proposer(state.next_height, 0)
    }

    fn is_validator(&self, address: &Address) -> bool {
        self.config.validators.contains(address)
    }

    async fn propose_block(
        &self,
        state: &ChainState,
        proposer_keypair: &Keypair,
        transactions: Vec<Transaction>,
    ) -> Result<Block, ConsensusError> {
        let proposer = proposer_keypair.address();
        let expected = self.config.proposer(state.next_height, 0);
        if proposer != expected {
            return Err(ConsensusError::NotLeader);
        }

        let mut block = BlockBuilder::new(
            state.head.hash(),
            state.next_height,
            state.timestamp,
            proposer,
        )
        .transactions(transactions)
        .build();

        // Authenticate the block: the proposer signs the header.
        block.header.sign_as_proposer(proposer_keypair);

        debug!(
            height = block.height(),
            hash = %block.hash(),
            "proposed BFT block"
        );

        Ok(block)
    }

    fn validate_block(&self, block: &Block, state: &ChainState) -> Result<(), ConsensusError> {
        // 1. Proposer must be a validator.
        if !self.is_validator(&block.header.proposer) {
            return Err(ConsensusError::InvalidBlock(format!(
                "proposer {} is not a validator",
                block.header.proposer
            )));
        }

        // 2. Proposer must be the expected leader for this height+round.
        let expected = self.config.proposer(state.next_height, 0);
        if block.header.proposer != expected {
            return Err(ConsensusError::InvalidBlock(format!(
                "wrong proposer for height {}: expected {}, got {}",
                block.height(),
                expected,
                block.header.proposer
            )));
        }

        // 3. Parent hash.
        if block.header.parent_hash != state.head.hash() {
            return Err(ConsensusError::InvalidBlock(
                "parent_hash does not match".into(),
            ));
        }

        // 4. Height.
        if block.height() != state.next_height {
            return Err(ConsensusError::InvalidBlock(format!(
                "wrong height: expected {}, got {}",
                state.next_height,
                block.height()
            )));
        }

        // 5. Tx root.
        if !block.validate_tx_root() {
            return Err(ConsensusError::InvalidBlock(
                "tx_root does not match transactions".into(),
            ));
        }

        // 6. Every transaction must carry a valid signature binding it to its
        // `from` address. Without this a forged transaction would be committed
        // behind a quorum certificate, i.e. finalized by the whole validator
        // set rather than merely accepted by one node.
        block
            .verify_transaction_signatures()
            .map_err(|(index, e)| {
                ConsensusError::InvalidBlock(format!(
                    "transaction at index {index} failed signature verification: {e:?}"
                ))
            })?;

        // 7. Protocol size limits: cap the per-block cost a proposer can
        //    impose on every validator and light client.
        block
            .validate_size_limits()
            .map_err(ConsensusError::InvalidBlock)?;

        // 8. The proposer must have signed the block.
        block.header.verify_proposer_signature().map_err(|e| {
            ConsensusError::InvalidBlock(format!("invalid proposer signature: {e:?}"))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_core::BlockHeader;
    use karoowa_crypto::{Hash, Keypair};

    fn addr(seed: u8) -> Address {
        Keypair::from_seed(&[seed; 32]).address()
    }

    /// The signing keypair for a validator address (test helper; validators
    /// are seeds 1..=4).
    fn keypair_for(a: Address) -> Keypair {
        (1u8..=4)
            .map(|n| Keypair::from_seed(&[n; 32]))
            .find(|kp| kp.address() == a)
            .expect("no validator keypair for address")
    }

    fn test_config() -> BFTConfig {
        BFTConfig {
            validators: vec![addr(1), addr(2), addr(3), addr(4)],
            propose_timeout_ms: 3000,
            prevote_timeout_ms: 1000,
            precommit_timeout_ms: 1000,
        }
    }

    fn genesis_state() -> ChainState {
        let proposer = addr(1);
        ChainState {
            head: BlockHeader {
                parent_hash: Hash::ZERO,
                state_root: Hash::ZERO,
                tx_root: Hash::ZERO,
                receipt_root: Hash::ZERO,
                height: 0,
                timestamp: 1700000000,
                proposer,
                consensus_data: vec![],
            },
            next_height: 1,
            timestamp: 1700000002,
        }
    }

    fn make_tx(nonce: u64) -> Transaction {
        let kp = Keypair::from_seed(&[1u8; 32]);
        let to = Address::from_public_key(&[99u8; 32]);
        Transaction::sign_transfer(&kp, to, 100, nonce, 1, 21000, 1)
    }

    #[test]
    fn engine_name() {
        let engine = BFTEngine::new(test_config()).unwrap();
        assert_eq!(engine.name(), "bft");
    }

    #[test]
    fn leader_rotation() {
        let engine = BFTEngine::new(test_config()).unwrap();
        let state = genesis_state();
        // Height 1, round 0 → validators[1] (index = 1 % 4)
        assert_eq!(engine.current_leader(&state), addr(2));
    }

    #[tokio::test]
    async fn validate_block_with_forged_tx_fails() {
        let engine = BFTEngine::new(test_config()).unwrap();
        let state = genesis_state();
        let leader = engine.current_leader(&state);

        // Without this check the forgery would be committed behind a quorum
        // certificate, i.e. finalized by the whole validator set.
        let mut forged = make_tx(0);
        forged.from = Address::from_public_key(&[7u8; 32]);

        let block = engine
            .propose_block(&state, &keypair_for(leader), vec![forged])
            .await
            .unwrap();

        assert!(engine.validate_block(&block, &state).is_err());
    }

    #[tokio::test]
    async fn propose_and_validate() {
        let engine = BFTEngine::new(test_config()).unwrap();
        let state = genesis_state();
        let leader = engine.current_leader(&state);

        let block = engine
            .propose_block(&state, &keypair_for(leader), vec![make_tx(0)])
            .await
            .unwrap();

        assert_eq!(block.height(), 1);
        assert!(engine.validate_block(&block, &state).is_ok());
    }

    #[tokio::test]
    async fn propose_as_non_leader_fails() {
        let engine = BFTEngine::new(test_config()).unwrap();
        let state = genesis_state();

        let wrong = keypair_for(addr(4)); // not the leader for height 1
        let result = engine.propose_block(&state, &wrong, vec![]).await;
        assert!(matches!(result, Err(ConsensusError::NotLeader)));
    }

    #[tokio::test]
    async fn full_round_commits() {
        let engine = BFTEngine::new(test_config()).unwrap();
        let state = genesis_state();
        let leader = engine.current_leader(&state);

        let (block, qc) = engine
            .run_round(&state, &keypair_for(leader), vec![make_tx(0)])
            .await
            .unwrap();

        assert_eq!(block.height(), 1);
        assert_eq!(qc.height, 1);
        assert_eq!(qc.round, 0);
        assert_eq!(qc.block_hash, block.hash());
        assert_eq!(qc.votes.len(), 4); // all validators voted
    }

    #[tokio::test]
    async fn multi_height_sequence() {
        let engine = BFTEngine::new(test_config()).unwrap();
        let mut current_head = genesis_state().head;

        for h in 1..=8u64 {
            let state = ChainState {
                head: current_head.clone(),
                next_height: h,
                timestamp: 1700000000 + h * 2,
            };
            let leader = engine.current_leader(&state);

            let (block, qc) = engine
                .run_round(&state, &keypair_for(leader), vec![make_tx(h)])
                .await
                .unwrap();

            assert_eq!(block.height(), h);
            assert_eq!(qc.votes.len(), 4);
            assert!(engine.validate_block(&block, &state).is_ok());
            current_head = block.header;
        }
    }

    #[tokio::test]
    async fn validate_wrong_proposer_fails() {
        let engine = BFTEngine::new(test_config()).unwrap();
        let state = genesis_state();
        let leader = engine.current_leader(&state);

        let mut block = engine
            .propose_block(&state, &keypair_for(leader), vec![])
            .await
            .unwrap();

        block.header.proposer = addr(4); // wrong proposer
        assert!(engine.validate_block(&block, &state).is_err());
    }

    #[test]
    fn one_validator_byzantine_still_commits() {
        // With 4 validators, quorum is 3. Even if 1 doesn't vote, quorum
        // is reachable with the remaining 3.
        let cfg = test_config();
        let engine = BFTEngine::new(cfg.clone()).unwrap();

        let mut round_state = engine.new_round(1, 0);
        let block_hash = Hash::ZERO;

        // Only 3 of 4 validators prevote.
        for v in &cfg.validators[0..3] {
            engine.prevote(&mut round_state, v, block_hash);
        }
        assert!(round_state.votes.has_prevote_quorum(&block_hash));

        // Only 3 precommit.
        for v in &cfg.validators[0..3] {
            engine.precommit(&mut round_state, v, block_hash);
        }
        assert!(round_state.votes.has_precommit_quorum(&block_hash));
        assert_eq!(round_state.step, Step::Committed);
    }

    #[test]
    fn two_byzantine_prevents_commit() {
        // With 4 validators and 2 not voting, only 2 votes — below quorum of 3.
        let cfg = test_config();
        let engine = BFTEngine::new(cfg.clone()).unwrap();

        let mut round_state = engine.new_round(1, 0);
        let block_hash = Hash::ZERO;

        // Only 2 of 4 prevote.
        for v in &cfg.validators[0..2] {
            engine.prevote(&mut round_state, v, block_hash);
        }
        assert!(!round_state.votes.has_prevote_quorum(&block_hash));
    }
}

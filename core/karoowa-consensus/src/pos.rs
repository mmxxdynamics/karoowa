//! Proof-of-Stake (PoS) consensus engine.
//!
//! Weighted leader selection proportional to stake. Validators must stake
//! tokens to participate. Block rewards are distributed to proposers.
//! Misbehaving validators are slashed and jailed.

use async_trait::async_trait;
use karoowa_core::{Block, BlockBuilder, Transaction, ValidatorSet};
use karoowa_crypto::{Address, Keypair};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use crate::engine::{ChainState, ConsensusEngine};
use crate::error::ConsensusError;

/// The PoS consensus engine.
pub struct PoSEngine {
    /// Shared validator set state — modified by staking transactions and
    /// reward distribution.
    validator_set: Arc<RwLock<ValidatorSet>>,
}

impl PoSEngine {
    /// Create a new PoS engine with the given initial validator set.
    pub fn new(validator_set: ValidatorSet) -> Result<Self, ConsensusError> {
        if validator_set.active_validators().is_empty() {
            return Err(ConsensusError::InvalidValidatorSet(
                "PoS requires at least one active validator".into(),
            ));
        }
        Ok(PoSEngine {
            validator_set: Arc::new(RwLock::new(validator_set)),
        })
    }

    /// Get a read handle to the validator set (for queries).
    pub fn validator_set(&self) -> Arc<RwLock<ValidatorSet>> {
        Arc::clone(&self.validator_set)
    }
}

#[async_trait]
impl ConsensusEngine for PoSEngine {
    fn name(&self) -> &'static str {
        "pos"
    }

    fn current_leader(&self, state: &ChainState) -> Address {
        // Use try_read — this succeeds unless a write lock is held,
        // which only happens briefly during propose_block.
        match self.validator_set.try_read() {
            Ok(vs) => vs
                .weighted_leader(state.next_height)
                .unwrap_or(Address::ZERO),
            Err(_) => Address::ZERO,
        }
    }

    fn is_validator(&self, address: &Address) -> bool {
        match self.validator_set.try_read() {
            Ok(vs) => vs
                .validators
                .get(address)
                .map(|v| v.is_active())
                .unwrap_or(false),
            Err(_) => false,
        }
    }

    async fn propose_block(
        &self,
        state: &ChainState,
        proposer_keypair: &Keypair,
        transactions: Vec<Transaction>,
    ) -> Result<Block, ConsensusError> {
        let proposer = proposer_keypair.address();
        let vs = self.validator_set.read().await;

        let expected_leader =
            vs.weighted_leader(state.next_height)
                .ok_or(ConsensusError::InvalidValidatorSet(
                    "no active validators".into(),
                ))?;

        if proposer != expected_leader {
            return Err(ConsensusError::NotLeader);
        }

        drop(vs);

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

        // Distribute reward to proposer.
        let mut vs = self.validator_set.write().await;
        vs.distribute_reward(&proposer);

        debug!(
            height = block.height(),
            hash = %block.hash(),
            proposer = %proposer,
            txs = block.transactions.len(),
            "proposed PoS block"
        );

        Ok(block)
    }

    fn validate_block(&self, block: &Block, state: &ChainState) -> Result<(), ConsensusError> {
        let vs = self
            .validator_set
            .try_read()
            .map_err(|_| ConsensusError::InvalidBlock("validator set lock contention".into()))?;

        // 1. Proposer must be an active validator.
        let validator = vs.validators.get(&block.header.proposer).ok_or_else(|| {
            ConsensusError::InvalidBlock(format!(
                "proposer {} is not a validator",
                block.header.proposer
            ))
        })?;

        if !validator.is_active() {
            return Err(ConsensusError::InvalidBlock(format!(
                "proposer {} is jailed or has zero stake",
                block.header.proposer
            )));
        }

        // 2. Proposer must be the weighted leader for this height.
        let expected =
            vs.weighted_leader(state.next_height)
                .ok_or(ConsensusError::InvalidValidatorSet(
                    "no active validators".into(),
                ))?;

        if block.header.proposer != expected {
            return Err(ConsensusError::InvalidBlock(format!(
                "wrong proposer for height {}: expected {}, got {}",
                block.height(),
                expected,
                block.header.proposer
            )));
        }

        // 3. Parent hash must match.
        if block.header.parent_hash != state.head.hash() {
            return Err(ConsensusError::InvalidBlock(
                "parent_hash does not match current head".into(),
            ));
        }

        // 4. Height must be head + 1.
        if block.height() != state.next_height {
            return Err(ConsensusError::InvalidBlock(format!(
                "wrong height: expected {}, got {}",
                state.next_height,
                block.height()
            )));
        }

        // 5. Tx root must match.
        if !block.validate_tx_root() {
            return Err(ConsensusError::InvalidBlock(
                "tx_root does not match transactions".into(),
            ));
        }

        // 6. Every transaction must carry a valid signature binding it to its
        // `from` address. A block body is attacker-controlled, so the elected
        // leader must not be trusted to have gone through the mempool.
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
    use karoowa_core::{BlockHeader, ValidatorSet};
    use karoowa_crypto::{Hash, Keypair};

    fn addr(seed: u8) -> Address {
        Keypair::from_seed(&[seed; 32]).address()
    }

    /// The signing keypair for a validator address (test helper; validators
    /// are seeds 1 and 2).
    fn keypair_for(a: Address) -> Keypair {
        [1u8, 2u8]
            .into_iter()
            .map(|n| Keypair::from_seed(&[n; 32]))
            .find(|kp| kp.address() == a)
            .expect("no validator keypair for address")
    }

    fn test_validator_set() -> ValidatorSet {
        let mut vs = ValidatorSet::new(100, 10);
        vs.add_validator(addr(1), 1000, 0).unwrap();
        vs.add_validator(addr(2), 3000, 0).unwrap();
        vs
    }

    fn genesis_state(vs: &ValidatorSet) -> ChainState {
        let proposer = vs.weighted_leader(0).unwrap();
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
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs).unwrap();
        assert_eq!(engine.name(), "pos");
    }

    #[test]
    fn empty_set_rejected() {
        let vs = ValidatorSet::new(100, 10);
        assert!(PoSEngine::new(vs).is_err());
    }

    #[test]
    fn leader_selection_is_weighted() {
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs).unwrap();

        // addr(2) has 3x the stake of addr(1), so should be selected more often.
        let mut count_2 = 0;
        for h in 0..100 {
            let state = ChainState {
                head: BlockHeader {
                    parent_hash: Hash::ZERO,
                    state_root: Hash::ZERO,
                    tx_root: Hash::ZERO,
                    receipt_root: Hash::ZERO,
                    height: h,
                    timestamp: 0,
                    proposer: Address::ZERO,
                    consensus_data: vec![],
                },
                next_height: h + 1,
                timestamp: 0,
            };
            if engine.current_leader(&state) == addr(2) {
                count_2 += 1;
            }
        }
        assert!(count_2 > 60, "addr(2) selected {count_2}/100 times");
    }

    #[tokio::test]
    async fn propose_and_validate() {
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs.clone()).unwrap();
        let state = genesis_state(&vs);
        let leader = engine.current_leader(&state);

        let block = engine
            .propose_block(&state, &keypair_for(leader), vec![make_tx(0)])
            .await
            .unwrap();

        assert_eq!(block.height(), 1);
        assert_eq!(block.header.proposer, leader);
        assert!(engine.validate_block(&block, &state).is_ok());
    }

    #[tokio::test]
    async fn propose_as_non_leader_fails() {
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs.clone()).unwrap();
        let state = genesis_state(&vs);
        let leader = engine.current_leader(&state);

        // Pick the other validator's keypair (not the leader for this height).
        let wrong = if leader == addr(1) {
            keypair_for(addr(2))
        } else {
            keypair_for(addr(1))
        };
        let result = engine.propose_block(&state, &wrong, vec![]).await;
        assert!(matches!(result, Err(ConsensusError::NotLeader)));
    }

    #[tokio::test]
    async fn reward_distributed_on_propose() {
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs.clone()).unwrap();
        let state = genesis_state(&vs);
        let leader = engine.current_leader(&state);

        engine
            .propose_block(&state, &keypair_for(leader), vec![])
            .await
            .unwrap();

        let vs_arc = engine.validator_set();
        let vs_read = vs_arc.read().await;
        let validator = vs_read.validators.get(&leader).unwrap();
        assert_eq!(validator.pending_rewards, 10); // block_reward = 10
    }

    #[tokio::test]
    async fn validate_block_with_forged_tx_fails() {
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs.clone()).unwrap();
        let state = genesis_state(&vs);
        let leader = engine.current_leader(&state);

        // The mempool never saw this transaction: the elected leader put it
        // straight into the block. The signature is valid but `from` claims an
        // address the signing key does not control.
        let mut forged = make_tx(0);
        forged.from = Address::from_public_key(&[7u8; 32]);

        let block = engine
            .propose_block(&state, &keypair_for(leader), vec![forged])
            .await
            .unwrap();

        assert!(engine.validate_block(&block, &state).is_err());
    }

    #[tokio::test]
    async fn validate_block_rejects_forged_tx_after_valid_ones() {
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs.clone()).unwrap();
        let state = genesis_state(&vs);
        let leader = engine.current_leader(&state);

        // Pins that the check scans the whole body rather than stopping at the
        // first transaction.
        let mut forged = make_tx(2);
        forged.from = Address::from_public_key(&[7u8; 32]);

        let block = engine
            .propose_block(
                &state,
                &keypair_for(leader),
                vec![make_tx(0), make_tx(1), forged],
            )
            .await
            .unwrap();

        assert!(engine.validate_block(&block, &state).is_err());
    }

    #[tokio::test]
    async fn validate_wrong_proposer_fails() {
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs.clone()).unwrap();
        let state = genesis_state(&vs);
        let leader = engine.current_leader(&state);

        let mut block = engine
            .propose_block(&state, &keypair_for(leader), vec![])
            .await
            .unwrap();

        // Tamper proposer.
        let other = if leader == addr(1) { addr(2) } else { addr(1) };
        block.header.proposer = other;

        assert!(engine.validate_block(&block, &state).is_err());
    }

    #[tokio::test]
    async fn multi_block_sequence() {
        let vs = test_validator_set();
        let engine = PoSEngine::new(vs.clone()).unwrap();

        let mut current_head = genesis_state(&vs).head;

        for h in 1..=10u64 {
            let state = ChainState {
                head: current_head.clone(),
                next_height: h,
                timestamp: 1700000000 + h * 2,
            };
            let leader = engine.current_leader(&state);

            let block = engine
                .propose_block(&state, &keypair_for(leader), vec![make_tx(h)])
                .await
                .unwrap();

            assert!(engine.validate_block(&block, &state).is_ok());
            current_head = block.header;
        }
    }
}

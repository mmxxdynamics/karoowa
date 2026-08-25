//! Proof-of-Authority (PoA) consensus engine.
//!
//! A simple round-robin consensus where a fixed, ordered validator set takes
//! turns proposing blocks. The leader for height `h` is
//! `validators[h % validators.len()]`.
//!
//! This is Karoowa's M1 consensus engine — suitable for development and
//! permissioned networks. PoS and BFT engines follow in M2.

use async_trait::async_trait;
use karoowa_core::{Block, BlockBuilder, Transaction};
use karoowa_crypto::{Address, Keypair};
use tracing::debug;

use crate::engine::{ChainState, ConsensusEngine};
use crate::error::ConsensusError;

/// Configuration for the PoA engine.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoAConfig {
    /// Ordered list of validator addresses. Position in the list determines
    /// round-robin turn order.
    pub validators: Vec<Address>,
}

impl PoAConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConsensusError> {
        if self.validators.is_empty() {
            return Err(ConsensusError::InvalidValidatorSet(
                "PoA requires at least one validator".into(),
            ));
        }
        Ok(())
    }
}

/// The PoA consensus engine.
pub struct PoAEngine {
    config: PoAConfig,
}

impl PoAEngine {
    /// Create a new PoA engine with the given configuration.
    pub fn new(config: PoAConfig) -> Result<Self, ConsensusError> {
        config.validate()?;
        Ok(PoAEngine { config })
    }

    /// Determine the leader for a given block height using round-robin.
    fn leader_for_height(&self, height: u64) -> Address {
        let idx = (height as usize) % self.config.validators.len();
        self.config.validators[idx]
    }
}

#[async_trait]
impl ConsensusEngine for PoAEngine {
    fn name(&self) -> &'static str {
        "poa"
    }

    fn current_leader(&self, state: &ChainState) -> Address {
        self.leader_for_height(state.next_height)
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
        let expected_leader = self.current_leader(state);
        if proposer != expected_leader {
            debug!(
                proposer = %proposer,
                expected = %expected_leader,
                height = state.next_height,
                "not the current leader"
            );
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
            txs = block.transactions.len(),
            "proposed PoA block"
        );

        Ok(block)
    }

    fn validate_block(&self, block: &Block, state: &ChainState) -> Result<(), ConsensusError> {
        // 1. Proposer must be a known validator.
        if !self.is_validator(&block.header.proposer) {
            return Err(ConsensusError::InvalidBlock(format!(
                "proposer {} is not a validator",
                block.header.proposer
            )));
        }

        // 2. Proposer must be the expected leader for this height.
        let expected = self.leader_for_height(block.height());
        if block.header.proposer != expected {
            return Err(ConsensusError::InvalidBlock(format!(
                "wrong proposer for height {}: expected {}, got {}",
                block.height(),
                expected,
                block.header.proposer
            )));
        }

        // 3. Parent hash must match the current head.
        if block.header.parent_hash != state.head.hash() {
            return Err(ConsensusError::InvalidBlock(
                "parent_hash does not match current head".into(),
            ));
        }

        // 4. Height must be exactly head + 1.
        if block.height() != state.next_height {
            return Err(ConsensusError::InvalidBlock(format!(
                "wrong height: expected {}, got {}",
                state.next_height,
                block.height()
            )));
        }

        // 5. Tx root must match the block body.
        if !block.validate_tx_root() {
            return Err(ConsensusError::InvalidBlock(
                "tx_root does not match transactions".into(),
            ));
        }

        // 6. Every transaction must carry a valid signature binding it to
        // its `from` address. The mempool already rejects forgeries, but a
        // malicious or buggy proposer can put transactions straight into a
        // block, so validation must not trust the proposer's pool.
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

        // 8. The proposer must have signed the block (real authentication,
        //    replacing the old public consensus_data tag).
        block.header.verify_proposer_signature().map_err(|e| {
            ConsensusError::InvalidBlock(format!("invalid proposer signature: {e:?}"))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_core::{BlockHeader, Transaction};
    use karoowa_crypto::{sha3_256, Hash, Keypair};

    fn validator_keypairs() -> Vec<Keypair> {
        (0..4u8).map(|i| Keypair::from_seed(&[i + 1; 32])).collect()
    }

    fn test_validators() -> Vec<Address> {
        validator_keypairs().iter().map(|kp| kp.address()).collect()
    }

    /// The signing keypair for a validator address (test helper).
    fn keypair_for(addr: Address) -> Keypair {
        validator_keypairs()
            .into_iter()
            .find(|kp| kp.address() == addr)
            .expect("no keypair for validator address")
    }

    fn test_engine() -> PoAEngine {
        let config = PoAConfig {
            validators: test_validators(),
        };
        PoAEngine::new(config).unwrap()
    }

    fn genesis_state() -> (ChainState, BlockHeader) {
        let proposer = test_validators()[0];
        let header = BlockHeader {
            parent_hash: Hash::ZERO,
            state_root: Hash::ZERO,
            tx_root: Hash::ZERO,
            receipt_root: Hash::ZERO,
            height: 0,
            timestamp: 1700000000,
            proposer,
            consensus_data: vec![], // genesis is trusted, not signature-verified
        };
        let state = ChainState {
            head: header.clone(),
            next_height: 1,
            timestamp: 1700000002,
        };
        (state, header)
    }

    fn make_tx(nonce: u64) -> Transaction {
        let kp = Keypair::from_seed(&[1u8; 32]);
        let to = Address::from_public_key(&[99u8; 32]);
        Transaction::sign_transfer(&kp, to, 100, nonce, 1, 21000, 1)
    }

    #[test]
    fn round_robin_leader_selection() {
        let engine = test_engine();
        let validators = test_validators();

        for h in 0..12u64 {
            let state = ChainState {
                head: BlockHeader {
                    parent_hash: Hash::ZERO,
                    state_root: Hash::ZERO,
                    tx_root: Hash::ZERO,
                    receipt_root: Hash::ZERO,
                    height: h.saturating_sub(1),
                    timestamp: 0,
                    proposer: Address::ZERO,
                    consensus_data: vec![],
                },
                next_height: h,
                timestamp: 0,
            };
            let leader = engine.current_leader(&state);
            assert_eq!(leader, validators[(h as usize) % 4]);
        }
    }

    #[test]
    fn is_validator() {
        let engine = test_engine();
        let validators = test_validators();
        for v in &validators {
            assert!(engine.is_validator(v));
        }
        assert!(!engine.is_validator(&Address::ZERO));
    }

    #[tokio::test]
    async fn propose_block_as_leader() {
        let engine = test_engine();
        let (state, _) = genesis_state();
        let leader = engine.current_leader(&state);

        let block = engine
            .propose_block(&state, &keypair_for(leader), vec![make_tx(0)])
            .await
            .unwrap();

        assert_eq!(block.height(), 1);
        assert_eq!(block.header.proposer, leader);
        assert_eq!(block.transactions.len(), 1);
        assert!(block.validate_tx_root());
    }

    #[tokio::test]
    async fn propose_block_not_leader_fails() {
        let engine = test_engine();
        let (state, _) = genesis_state();
        let wrong = Keypair::from_seed(&[99u8; 32]); // address not in the validator set

        let result = engine.propose_block(&state, &wrong, vec![]).await;

        assert!(matches!(result, Err(ConsensusError::NotLeader)));
    }

    #[tokio::test]
    async fn validate_proposed_block() {
        let engine = test_engine();
        let (state, _) = genesis_state();
        let leader = engine.current_leader(&state);

        let block = engine
            .propose_block(&state, &keypair_for(leader), vec![make_tx(0)])
            .await
            .unwrap();

        assert!(engine.validate_block(&block, &state).is_ok());
    }

    #[tokio::test]
    async fn validate_block_with_forged_tx_fails() {
        let engine = test_engine();
        let (state, _) = genesis_state();
        let leader = engine.current_leader(&state);

        // The mempool never saw this transaction: a malicious proposer put
        // it straight into the block. The signature is valid but `from`
        // claims an address the signing key does not control.
        let mut forged = make_tx(0);
        forged.from = Address::from_public_key(&[7u8; 32]);

        let block = engine
            .propose_block(&state, &keypair_for(leader), vec![forged])
            .await
            .unwrap();

        assert!(engine.validate_block(&block, &state).is_err());
    }

    #[tokio::test]
    async fn validate_wrong_proposer_fails() {
        let engine = test_engine();
        let (state, _) = genesis_state();
        let leader = engine.current_leader(&state);

        let mut block = engine
            .propose_block(&state, &keypair_for(leader), vec![])
            .await
            .unwrap();

        // Tamper: change proposer to a different validator.
        block.header.proposer = test_validators()[2];

        let result = engine.validate_block(&block, &state);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn validate_wrong_parent_hash_fails() {
        let engine = test_engine();
        let (state, _) = genesis_state();
        let leader = engine.current_leader(&state);

        let mut block = engine
            .propose_block(&state, &keypair_for(leader), vec![])
            .await
            .unwrap();

        block.header.parent_hash = sha3_256(b"wrong");

        let result = engine.validate_block(&block, &state);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn validate_wrong_height_fails() {
        let engine = test_engine();
        let (state, _) = genesis_state();
        let leader = engine.current_leader(&state);

        let mut block = engine
            .propose_block(&state, &keypair_for(leader), vec![])
            .await
            .unwrap();

        block.header.height = 999;

        let result = engine.validate_block(&block, &state);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn validate_tampered_tx_root_fails() {
        let engine = test_engine();
        let (state, _) = genesis_state();
        let leader = engine.current_leader(&state);

        let mut block = engine
            .propose_block(&state, &keypair_for(leader), vec![make_tx(0)])
            .await
            .unwrap();

        block.header.tx_root = Hash::ZERO; // tamper

        let result = engine.validate_block(&block, &state);
        assert!(result.is_err());
    }

    #[test]
    fn empty_validator_set_is_rejected() {
        let config = PoAConfig { validators: vec![] };
        assert!(PoAEngine::new(config).is_err());
    }

    #[tokio::test]
    async fn multi_block_sequence() {
        let engine = test_engine();
        let validators = test_validators();

        // Build genesis header.
        let genesis_proposer = validators[0];
        let genesis_header = BlockHeader {
            parent_hash: Hash::ZERO,
            state_root: Hash::ZERO,
            tx_root: Hash::ZERO,
            receipt_root: Hash::ZERO,
            height: 0,
            timestamp: 1700000000,
            proposer: genesis_proposer,
            consensus_data: vec![], // genesis is trusted, not signature-verified
        };

        let mut current_head = genesis_header;

        for h in 1..=8u64 {
            let state = ChainState {
                head: current_head.clone(),
                next_height: h,
                timestamp: 1700000000 + h * 2,
            };
            let leader = engine.current_leader(&state);
            assert_eq!(leader, validators[(h as usize) % 4]);

            let block = engine
                .propose_block(&state, &keypair_for(leader), vec![make_tx(h)])
                .await
                .unwrap();

            assert!(engine.validate_block(&block, &state).is_ok());
            assert_eq!(block.header.parent_hash, current_head.hash());
            current_head = block.header;
        }
    }

    #[test]
    fn engine_name() {
        let engine = test_engine();
        assert_eq!(engine.name(), "poa");
    }
}

//! Block producer — the tokio task that drives block production.
//!
//! The [`BlockProducer`] runs as a long-lived async task. On each tick (at the
//! configured block interval), it checks whether this node is the current
//! leader and, if so, proposes a block from the pending transactions, stores
//! it, and broadcasts it via a channel.

use karoowa_core::{Block, BlockHeader, Transaction};
use karoowa_crypto::Address;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::engine::{ChainState, ConsensusEngine};
use crate::error::ConsensusError;

/// A handle to the block producer that receives newly produced blocks.
pub type BlockReceiver = mpsc::Receiver<Block>;

/// Configuration for the block producer.
pub struct ProducerConfig {
    /// This node's validator address.
    pub validator_address: Address,
    /// Target block interval.
    pub block_interval: Duration,
}

/// The block producer task.
///
/// Drives the propose → validate → broadcast loop. Designed to be spawned
/// as a tokio task via [`BlockProducer::run`].
pub struct BlockProducer<E: ConsensusEngine> {
    engine: Arc<E>,
    config: ProducerConfig,
    /// Shared mutable head — updated after each successful block.
    head: Arc<Mutex<BlockHeader>>,
    /// Pending transactions to include in the next block.
    pending_txs: Arc<Mutex<Vec<Transaction>>>,
    /// Channel to send produced blocks to the rest of the node.
    block_tx: mpsc::Sender<Block>,
}

impl<E: ConsensusEngine + 'static> BlockProducer<E> {
    /// Create a new block producer.
    ///
    /// Returns the producer and a receiver that will yield each newly
    /// produced block.
    pub fn new(
        engine: Arc<E>,
        config: ProducerConfig,
        genesis_header: BlockHeader,
    ) -> (Self, BlockReceiver) {
        let (block_tx, block_rx) = mpsc::channel(16);
        let producer = BlockProducer {
            engine,
            config,
            head: Arc::new(Mutex::new(genesis_header)),
            pending_txs: Arc::new(Mutex::new(Vec::new())),
            block_tx,
        };
        (producer, block_rx)
    }

    /// Get a handle to submit pending transactions.
    pub fn pending_tx_sender(&self) -> PendingTxSender {
        PendingTxSender {
            txs: Arc::clone(&self.pending_txs),
        }
    }

    /// Get a handle to update the head (e.g. when receiving a block from the
    /// network that's newer than our own production).
    pub fn head_handle(&self) -> HeadHandle {
        HeadHandle {
            head: Arc::clone(&self.head),
        }
    }

    /// Run the block production loop. This never returns under normal
    /// operation — cancel the task to stop it.
    pub async fn run(self) -> Result<(), ConsensusError> {
        let mut interval = tokio::time::interval(self.config.block_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!(
            engine = self.engine.name(),
            validator = %self.config.validator_address,
            interval_ms = self.config.block_interval.as_millis() as u64,
            "block producer started"
        );

        loop {
            interval.tick().await;

            match self.try_produce_block().await {
                Ok(Some(block)) => {
                    if self.block_tx.send(block).await.is_err() {
                        debug!("block receiver dropped, stopping producer");
                        return Ok(());
                    }
                }
                Ok(None) => {
                    // Not our turn — skip.
                }
                Err(e) => {
                    warn!(error = %e, "block production failed");
                }
            }
        }
    }

    /// Attempt to produce one block. Returns `Ok(None)` if we're not the
    /// current leader.
    async fn try_produce_block(&self) -> Result<Option<Block>, ConsensusError> {
        let head = self.head.lock().await;
        let next_height = head.height + 1;

        let state = ChainState {
            head: head.clone(),
            next_height,
            timestamp: now_secs(),
        };

        let leader = self.engine.current_leader(&state);
        if leader != self.config.validator_address {
            return Ok(None);
        }

        // Drain pending transactions.
        let txs = {
            let mut pending = self.pending_txs.lock().await;
            std::mem::take(&mut *pending)
        };

        // Drop the head lock before the async propose call.
        drop(head);

        let block = self
            .engine
            .propose_block(&state, &self.config.validator_address, txs)
            .await?;

        // Update the head.
        let mut head = self.head.lock().await;
        *head = block.header.clone();

        info!(
            height = block.height(),
            hash = %block.hash(),
            txs = block.transactions.len(),
            "produced block"
        );

        Ok(Some(block))
    }
}

/// Handle for submitting transactions to the producer's pending pool.
#[derive(Clone)]
pub struct PendingTxSender {
    txs: Arc<Mutex<Vec<Transaction>>>,
}

impl PendingTxSender {
    /// Add a transaction to the pending pool.
    pub async fn submit(&self, tx: Transaction) {
        self.txs.lock().await.push(tx);
    }
}

/// Handle for updating the producer's head (e.g. on receiving a new block
/// from the network).
#[derive(Clone)]
pub struct HeadHandle {
    head: Arc<Mutex<BlockHeader>>,
}

impl HeadHandle {
    /// Update the head to a new block header.
    pub async fn set_head(&self, header: BlockHeader) {
        *self.head.lock().await = header;
    }
}

/// Current time in seconds since UNIX epoch.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poa::{PoAConfig, PoAEngine};
    use karoowa_crypto::{Hash, Keypair};

    fn test_setup() -> (Arc<PoAEngine>, ProducerConfig, BlockHeader, Vec<Keypair>) {
        let keypairs: Vec<Keypair> = (0..4u8).map(|i| Keypair::from_seed(&[i + 1; 32])).collect();
        let validators: Vec<Address> = keypairs.iter().map(|kp| kp.address()).collect();

        let config = PoAConfig {
            validators: validators.clone(),
        };
        let engine = Arc::new(PoAEngine::new(config).unwrap());

        // Genesis block proposed by validator 0.
        let genesis_header = BlockHeader {
            parent_hash: Hash::ZERO,
            state_root: Hash::ZERO,
            tx_root: Hash::ZERO,
            receipt_root: Hash::ZERO,
            height: 0,
            timestamp: 1700000000,
            proposer: validators[0],
            consensus_data: vec![], // simplified for genesis
        };

        // Height 1 leader is validators[1].
        let producer_config = ProducerConfig {
            validator_address: validators[1],
            block_interval: Duration::from_millis(50),
        };

        (engine, producer_config, genesis_header, keypairs)
    }

    #[tokio::test]
    async fn produces_a_block_when_leader() {
        let (engine, config, genesis, keypairs) = test_setup();
        let (producer, mut block_rx) = BlockProducer::new(engine, config, genesis);

        // Submit a transaction.
        let tx_sender = producer.pending_tx_sender();
        let to = Address::from_public_key(&[99u8; 32]);
        let tx = Transaction::sign_transfer(&keypairs[0], to, 100, 0, 1, 21000, 1);
        tx_sender.submit(tx).await;

        // Run the producer in the background and wait for the first block.
        let handle = tokio::spawn(producer.run());

        let block = tokio::time::timeout(Duration::from_secs(2), block_rx.recv())
            .await
            .expect("timed out waiting for block")
            .expect("channel closed");

        assert_eq!(block.height(), 1);
        assert_eq!(block.transactions.len(), 1);

        handle.abort();
    }

    #[tokio::test]
    async fn pending_tx_sender_is_cloneable() {
        let (engine, config, genesis, _) = test_setup();
        let (producer, _rx) = BlockProducer::new(engine, config, genesis);

        let sender1 = producer.pending_tx_sender();
        let sender2 = sender1.clone();

        // Both should work.
        let kp = Keypair::from_seed(&[1u8; 32]);
        let to = Address::ZERO;
        sender1
            .submit(Transaction::sign_transfer(&kp, to, 1, 0, 1, 21000, 1))
            .await;
        sender2
            .submit(Transaction::sign_transfer(&kp, to, 2, 1, 1, 21000, 1))
            .await;
    }

    #[tokio::test]
    async fn head_handle_updates_head() {
        let (engine, config, genesis, _) = test_setup();
        let (producer, _rx) = BlockProducer::new(engine, config, genesis.clone());

        let head_handle = producer.head_handle();
        let mut new_head = genesis;
        new_head.height = 42;
        head_handle.set_head(new_head).await;
    }
}

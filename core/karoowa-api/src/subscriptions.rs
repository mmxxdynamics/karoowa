//! WebSocket subscription manager.
//!
//! Tracks active subscriptions by ID, fans out events from consensus
//! and mempool to connected WebSocket clients. Supports three event types:
//!
//! - `newBlocks` — emits block headers when a new block is produced
//! - `pendingTransactions` — emits tx hashes when a tx enters the mempool
//! - `logs` — emits logs matching a filter (address + topics)

use karoowa_core::{Block, Log, Transaction};
use karoowa_crypto::{Address, Hash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::debug;

/// A subscription event sent to WebSocket clients.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum SubscriptionEvent {
    /// A new block was produced.
    NewBlock {
        height: u64,
        hash: String,
        timestamp: u64,
        tx_count: usize,
    },
    /// A new transaction entered the mempool.
    PendingTransaction { hash: String },
    /// A log matching the subscription filter was emitted.
    Log {
        address: String,
        topics: Vec<String>,
        data: String,
    },
}

/// Filter for log subscriptions.
#[derive(Debug, Clone, Deserialize)]
pub struct LogFilter {
    /// Contract address to filter on (None = all addresses).
    pub address: Option<Address>,
    /// Topic filters (None entries are wildcards).
    pub topics: Vec<Option<Hash>>,
}

/// Subscription type requested by a client.
#[derive(Debug, Clone)]
enum SubscriptionKind {
    NewBlocks,
    PendingTransactions,
    Logs(LogFilter),
}

/// Internal subscription record.
struct Subscription {
    kind: SubscriptionKind,
    sender: mpsc::Sender<SubscriptionEvent>,
}

/// Manages all active WebSocket subscriptions.
///
/// Thread-safe and cheaply cloneable via `Arc` internals.
#[derive(Clone)]
pub struct SubscriptionManager {
    inner: Arc<SubscriptionManagerInner>,
}

struct SubscriptionManagerInner {
    next_id: AtomicU64,
    subscriptions: RwLock<HashMap<u64, Subscription>>,
}

/// Handle returned to the client for receiving events.
pub struct SubscriptionHandle {
    pub id: u64,
    pub receiver: mpsc::Receiver<SubscriptionEvent>,
}

impl SubscriptionManager {
    /// Create a new subscription manager.
    pub fn new() -> Self {
        SubscriptionManager {
            inner: Arc::new(SubscriptionManagerInner {
                next_id: AtomicU64::new(1),
                subscriptions: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Subscribe to new blocks. Returns a subscription ID and a receiver.
    pub async fn subscribe_new_blocks(&self) -> SubscriptionHandle {
        self.add_subscription(SubscriptionKind::NewBlocks).await
    }

    /// Subscribe to pending transactions.
    pub async fn subscribe_pending_transactions(&self) -> SubscriptionHandle {
        self.add_subscription(SubscriptionKind::PendingTransactions)
            .await
    }

    /// Subscribe to logs matching a filter.
    pub async fn subscribe_logs(&self, filter: LogFilter) -> SubscriptionHandle {
        self.add_subscription(SubscriptionKind::Logs(filter)).await
    }

    /// Remove a subscription by ID.
    pub async fn unsubscribe(&self, id: u64) -> bool {
        self.inner.subscriptions.write().await.remove(&id).is_some()
    }

    /// Notify all subscribers about a new block.
    pub async fn notify_new_block(&self, block: &Block) {
        let event = SubscriptionEvent::NewBlock {
            height: block.height(),
            hash: block.hash().to_string(),
            timestamp: block.header.timestamp,
            tx_count: block.transactions.len(),
        };

        let subs = self.inner.subscriptions.read().await;
        for (id, sub) in subs.iter() {
            if matches!(sub.kind, SubscriptionKind::NewBlocks)
                && sub.sender.try_send(event.clone()).is_err()
            {
                debug!(sub_id = id, "dropping slow subscriber (newBlocks)");
            }
        }
    }

    /// Notify all subscribers about a pending transaction.
    pub async fn notify_pending_tx(&self, tx: &Transaction) {
        let event = SubscriptionEvent::PendingTransaction {
            hash: tx.hash().to_string(),
        };

        let subs = self.inner.subscriptions.read().await;
        for (id, sub) in subs.iter() {
            if matches!(sub.kind, SubscriptionKind::PendingTransactions)
                && sub.sender.try_send(event.clone()).is_err()
            {
                debug!(sub_id = id, "dropping slow subscriber (pendingTx)");
            }
        }
    }

    /// Notify all subscribers about logs from a block's receipts.
    pub async fn notify_logs(&self, logs: &[(Address, Log)]) {
        let subs = self.inner.subscriptions.read().await;
        for (address, log) in logs {
            let event = SubscriptionEvent::Log {
                address: address.to_string(),
                topics: log.topics.iter().map(|t| t.to_string()).collect(),
                data: hex::encode(&log.data),
            };

            for (id, sub) in subs.iter() {
                if let SubscriptionKind::Logs(ref filter) = sub.kind {
                    if matches_filter(address, log, filter)
                        && sub.sender.try_send(event.clone()).is_err()
                    {
                        debug!(sub_id = id, "dropping slow subscriber (logs)");
                    }
                }
            }
        }
    }

    /// Number of active subscriptions.
    pub async fn active_count(&self) -> usize {
        self.inner.subscriptions.read().await.len()
    }

    async fn add_subscription(&self, kind: SubscriptionKind) -> SubscriptionHandle {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        // Buffer 64 events per subscriber; excess is dropped (backpressure).
        let (tx, rx) = mpsc::channel(64);
        let sub = Subscription { kind, sender: tx };
        self.inner.subscriptions.write().await.insert(id, sub);
        debug!(sub_id = id, "new subscription");
        SubscriptionHandle { id, receiver: rx }
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a log matches a subscription filter.
fn matches_filter(address: &Address, log: &Log, filter: &LogFilter) -> bool {
    // Address filter.
    if let Some(ref filter_addr) = filter.address {
        if address != filter_addr {
            return false;
        }
    }
    // Topic filters.
    for (i, topic_filter) in filter.topics.iter().enumerate() {
        if let Some(ref expected) = topic_filter {
            match log.topics.get(i) {
                Some(actual) if actual == expected => {}
                _ => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_core::BlockBuilder;
    use karoowa_crypto::Hash;

    #[tokio::test]
    async fn subscribe_and_receive_block() {
        let mgr = SubscriptionManager::new();
        let mut handle = mgr.subscribe_new_blocks().await;

        let block = BlockBuilder::new(Hash::ZERO, 0, 1700000000, Address::ZERO).build();
        mgr.notify_new_block(&block).await;

        let event = handle.receiver.recv().await.unwrap();
        match event {
            SubscriptionEvent::NewBlock { height, .. } => assert_eq!(height, 0),
            _ => panic!("expected NewBlock event"),
        }
    }

    #[tokio::test]
    async fn subscribe_pending_tx() {
        let mgr = SubscriptionManager::new();
        let mut handle = mgr.subscribe_pending_transactions().await;

        let kp = karoowa_crypto::Keypair::from_seed(&[1u8; 32]);
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = karoowa_core::Transaction::sign_transfer(&kp, to, 100, 0, 1, 21000, 1);
        let expected_hash = tx.hash().to_string();

        mgr.notify_pending_tx(&tx).await;

        let event = handle.receiver.recv().await.unwrap();
        match event {
            SubscriptionEvent::PendingTransaction { hash } => assert_eq!(hash, expected_hash),
            _ => panic!("expected PendingTransaction event"),
        }
    }

    #[tokio::test]
    async fn unsubscribe() {
        let mgr = SubscriptionManager::new();
        let handle = mgr.subscribe_new_blocks().await;
        assert_eq!(mgr.active_count().await, 1);

        assert!(mgr.unsubscribe(handle.id).await);
        assert_eq!(mgr.active_count().await, 0);

        assert!(!mgr.unsubscribe(999).await); // non-existent
    }

    #[tokio::test]
    async fn multiple_subscribers() {
        let mgr = SubscriptionManager::new();
        let mut h1 = mgr.subscribe_new_blocks().await;
        let mut h2 = mgr.subscribe_new_blocks().await;

        let block = BlockBuilder::new(Hash::ZERO, 5, 1700000000, Address::ZERO).build();
        mgr.notify_new_block(&block).await;

        // Both should receive the event.
        let e1 = h1.receiver.recv().await.unwrap();
        let e2 = h2.receiver.recv().await.unwrap();
        match (e1, e2) {
            (
                SubscriptionEvent::NewBlock { height: h1, .. },
                SubscriptionEvent::NewBlock { height: h2, .. },
            ) => {
                assert_eq!(h1, 5);
                assert_eq!(h2, 5);
            }
            _ => panic!("expected NewBlock events"),
        }
    }

    #[tokio::test]
    async fn log_filter_matches() {
        let mgr = SubscriptionManager::new();
        let addr = Address::from_public_key(&[1u8; 32]);
        let topic = karoowa_crypto::sha3_256(b"Transfer");

        let filter = LogFilter {
            address: Some(addr),
            topics: vec![Some(topic)],
        };
        let mut handle = mgr.subscribe_logs(filter).await;

        let log = Log {
            address: addr,
            topics: vec![topic],
            data: vec![1, 2, 3],
        };
        mgr.notify_logs(&[(addr, log)]).await;

        let event = handle.receiver.recv().await.unwrap();
        assert!(matches!(event, SubscriptionEvent::Log { .. }));
    }

    #[tokio::test]
    async fn log_filter_rejects_wrong_address() {
        let mgr = SubscriptionManager::new();
        let filter_addr = Address::from_public_key(&[1u8; 32]);
        let other_addr = Address::from_public_key(&[2u8; 32]);

        let filter = LogFilter {
            address: Some(filter_addr),
            topics: vec![],
        };
        let mut handle = mgr.subscribe_logs(filter).await;

        let log = Log {
            address: other_addr,
            topics: vec![],
            data: vec![],
        };
        mgr.notify_logs(&[(other_addr, log)]).await;

        // Should not receive — wrong address.
        assert!(handle.receiver.try_recv().is_err());
    }
}

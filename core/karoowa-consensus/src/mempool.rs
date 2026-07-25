//! Transaction mempool.
//!
//! Stores pending transactions sorted by gas price (highest first), indexed
//! by hash and by sender+nonce. Enforces size limits, expiry, and
//! replace-by-fee policy.
//!
//! The mempool is the source of transactions for the block proposer and the
//! `kw_pendingTransactions` RPC method.

use karoowa_core::Transaction;
use karoowa_crypto::{Address, Hash};
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};
use tracing::debug;

/// Mempool configuration.
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions in the pool.
    pub max_size: usize,
    /// Transaction expiry time. Transactions older than this are evicted.
    pub tx_expiry: Duration,
    /// Minimum gas price multiplier for replace-by-fee (e.g. 1.1 = 10% bump).
    pub rbf_price_bump_pct: u64,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        MempoolConfig {
            max_size: 4096,
            tx_expiry: Duration::from_secs(3600), // 1 hour
            rbf_price_bump_pct: 10,               // 10% bump required
        }
    }
}

/// Reason a transaction was rejected from the mempool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// Duplicate transaction hash already in pool.
    AlreadyKnown,
    /// Pool is full and this transaction's gas price is too low to evict anything.
    PoolFull,
    /// Replace-by-fee: new tx has same sender+nonce but insufficient gas price bump.
    InsufficientGasBump {
        existing_price: u64,
        required_price: u64,
    },
    /// Transaction failed pre-validation.
    Invalid(String),
}

/// Entry wrapping a transaction with insertion metadata.
#[derive(Debug, Clone)]
struct PoolEntry {
    tx: Transaction,
    sender: Address,
    inserted_at: Instant,
}

/// The transaction mempool.
pub struct Mempool {
    config: MempoolConfig,
    /// All transactions by hash.
    by_hash: HashMap<Hash, PoolEntry>,
    /// Index: sender -> nonce -> tx hash. Used for replace-by-fee and nonce ordering.
    by_sender: HashMap<Address, BTreeMap<u64, Hash>>,
    /// Sorted set of (gas_price, tx_hash) for ordering. BTreeMap in reverse
    /// (negated price) so highest gas price comes first.
    by_price: BTreeMap<(std::cmp::Reverse<u64>, Hash), ()>,
}

impl Mempool {
    /// Create a new mempool with the given configuration.
    pub fn new(config: MempoolConfig) -> Self {
        Mempool {
            config,
            by_hash: HashMap::new(),
            by_sender: HashMap::new(),
            by_price: BTreeMap::new(),
        }
    }

    /// Number of transactions in the pool.
    pub fn len(&self) -> usize {
        self.by_hash.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    /// Insert a transaction into the mempool.
    ///
    /// Returns `Ok(())` if accepted, or `Err(RejectReason)` if rejected.
    pub fn insert(&mut self, tx: Transaction) -> Result<(), RejectReason> {
        // 0. Reject forgeries. Nothing upstream validates, so without this
        // check `tx.from` is a free-form claim and anyone could submit a
        // transaction "from" any address.
        //
        // This covers the RPC path (`kw_sendRawTransaction` -> `MempoolHandle`),
        // but it is NOT the only way a transaction reaches a block:
        // `PendingTxSender::submit` pushes straight into the producer's pending
        // queue without touching the mempool. Block validation therefore
        // re-verifies every signature via `Block::verify_transaction_signatures`
        // rather than trusting that a transaction came through here.
        //
        // Verification deliberately stays ahead of the replace-by-fee branch
        // below, which evicts an existing transaction. Verifying after it would
        // let an unsigned transaction with a victim's sender+nonce and a small
        // gas bump evict the victim's legitimate transaction before being
        // rejected itself.
        if let Err(e) = tx.verify_signature() {
            return Err(RejectReason::Invalid(format!(
                "signature verification failed: {e:?}"
            )));
        }

        // 0b. Reject oversized transactions. MAX_TX_BYTES is well below the
        //     block body limit, so anything admitted here always fits in a
        //     block — an oversized tx must never wedge block building.
        let size = tx.encoded_size();
        if size > karoowa_core::MAX_TX_BYTES {
            return Err(RejectReason::Invalid(format!(
                "transaction is {size} bytes, limit is {}",
                karoowa_core::MAX_TX_BYTES
            )));
        }

        let hash = tx.hash();
        let sender = tx.from;
        let nonce = tx.nonce;
        let gas_price = tx.gas_price;

        // 1. Reject duplicates.
        if self.by_hash.contains_key(&hash) {
            return Err(RejectReason::AlreadyKnown);
        }

        // 2. Check replace-by-fee: same sender + nonce.
        if let Some(nonce_map) = self.by_sender.get(&sender) {
            if let Some(existing_hash) = nonce_map.get(&nonce) {
                let existing = &self.by_hash[existing_hash];
                let required_price = existing.tx.gas_price
                    + (existing.tx.gas_price * self.config.rbf_price_bump_pct / 100).max(1);
                if gas_price < required_price {
                    return Err(RejectReason::InsufficientGasBump {
                        existing_price: existing.tx.gas_price,
                        required_price,
                    });
                }
                // RBF accepted — remove old tx.
                let old_hash = *existing_hash;
                self.remove_internal(&old_hash);
                debug!(
                    old_hash = %old_hash,
                    new_hash = %hash,
                    "replace-by-fee accepted"
                );
            }
        }

        // 3. Evict if full (drop lowest gas price tx).
        if self.by_hash.len() >= self.config.max_size {
            // The last entry in by_price has the lowest gas price (highest Reverse value).
            if let Some((&(std::cmp::Reverse(lowest_price), lowest_hash), _)) =
                self.by_price.last_key_value()
            {
                if gas_price <= lowest_price {
                    return Err(RejectReason::PoolFull);
                }
                self.remove_internal(&lowest_hash);
                debug!(evicted = %lowest_hash, "evicted lowest-price tx to make room");
            }
        }

        // 4. Insert.
        let entry = PoolEntry {
            tx,
            sender,
            inserted_at: Instant::now(),
        };

        self.by_price
            .insert((std::cmp::Reverse(gas_price), hash), ());
        self.by_sender
            .entry(sender)
            .or_default()
            .insert(nonce, hash);
        self.by_hash.insert(hash, entry);

        Ok(())
    }

    /// Remove a transaction by hash. Returns the transaction if it was present.
    pub fn remove(&mut self, hash: &Hash) -> Option<Transaction> {
        self.remove_internal(hash)
    }

    fn remove_internal(&mut self, hash: &Hash) -> Option<Transaction> {
        let entry = self.by_hash.remove(hash)?;
        self.by_price
            .remove(&(std::cmp::Reverse(entry.tx.gas_price), *hash));
        if let Some(nonce_map) = self.by_sender.get_mut(&entry.sender) {
            nonce_map.remove(&entry.tx.nonce);
            if nonce_map.is_empty() {
                self.by_sender.remove(&entry.sender);
            }
        }
        Some(entry.tx)
    }

    /// Remove all transactions with the given hashes (e.g. after a block is mined).
    pub fn remove_batch(&mut self, hashes: &[Hash]) {
        for hash in hashes {
            self.remove_internal(hash);
        }
    }

    /// Get a transaction by hash.
    pub fn get(&self, hash: &Hash) -> Option<&Transaction> {
        self.by_hash.get(hash).map(|e| &e.tx)
    }

    /// Check if a transaction hash is in the pool.
    pub fn contains(&self, hash: &Hash) -> bool {
        self.by_hash.contains_key(hash)
    }

    /// Return up to `limit` transactions sorted by gas price (highest first),
    /// suitable for block proposal.
    pub fn pending_sorted(&self, limit: usize) -> Vec<Transaction> {
        self.by_price
            .keys()
            .take(limit)
            .filter_map(|(_, hash)| self.by_hash.get(hash).map(|e| e.tx.clone()))
            .collect()
    }

    /// Return all pending transaction hashes.
    pub fn pending_hashes(&self) -> Vec<Hash> {
        self.by_hash.keys().copied().collect()
    }

    /// Evict expired transactions. Returns the number of evicted txs.
    pub fn evict_expired(&mut self) -> usize {
        let now = Instant::now();
        let expired: Vec<Hash> = self
            .by_hash
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.inserted_at) > self.config.tx_expiry)
            .map(|(hash, _)| *hash)
            .collect();

        let count = expired.len();
        for hash in &expired {
            self.remove_internal(hash);
        }
        if count > 0 {
            debug!(count, "evicted expired transactions");
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_crypto::Keypair;

    fn make_tx(kp: &Keypair, nonce: u64, gas_price: u64) -> Transaction {
        let to = Address::from_public_key(&[2u8; 32]);
        Transaction::sign_transfer(kp, to, 100, nonce, gas_price, 21000, 1)
    }

    #[test]
    fn insert_and_get() {
        let mut pool = Mempool::new(MempoolConfig::default());
        let kp = Keypair::from_seed(&[1u8; 32]);
        let tx = make_tx(&kp, 0, 10);
        let hash = tx.hash();

        assert!(pool.insert(tx).is_ok());
        assert_eq!(pool.len(), 1);
        assert!(pool.contains(&hash));
        assert!(pool.get(&hash).is_some());
    }

    #[test]
    fn garbage_signature_is_rejected() {
        let mut pool = Mempool::new(MempoolConfig::default());
        let kp = Keypair::from_seed(&[1u8; 32]);
        let mut tx = make_tx(&kp, 0, 10);
        tx.signature = vec![0u8; 64];

        assert!(matches!(pool.insert(tx), Err(RejectReason::Invalid(_))));
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn invalid_signature_cannot_evict_via_replace_by_fee() {
        // Regression guard for the check ordering in `insert`. Signature
        // verification must run before the replace-by-fee branch: if it ran
        // after, this unsigned transaction would evict the victim's legitimate
        // one on its way to being rejected, giving an attacker a free eviction
        // primitive that costs them nothing.
        let mut pool = Mempool::new(MempoolConfig::default());
        let victim_kp = Keypair::from_seed(&[1u8; 32]);

        let legit = make_tx(&victim_kp, 0, 10);
        let legit_hash = legit.hash();
        pool.insert(legit).unwrap();

        // Same sender + nonce, gas price bumped well past the RBF threshold,
        // but the signature is garbage.
        let mut attacker = make_tx(&victim_kp, 0, 100);
        attacker.signature = vec![0u8; 64];

        assert!(matches!(
            pool.insert(attacker),
            Err(RejectReason::Invalid(_))
        ));
        assert_eq!(pool.len(), 1);
        assert!(pool.get(&legit_hash).is_some());
    }

    #[test]
    fn forged_from_address_is_rejected() {
        let mut pool = Mempool::new(MempoolConfig::default());
        let kp = Keypair::from_seed(&[1u8; 32]);
        let mut tx = make_tx(&kp, 0, 10);
        // Valid signature, but claims to be from an address the signing
        // key does not control.
        tx.from = Address::from_public_key(&[7u8; 32]);

        assert!(matches!(pool.insert(tx), Err(RejectReason::Invalid(_))));
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn reject_duplicate() {
        let mut pool = Mempool::new(MempoolConfig::default());
        let kp = Keypair::from_seed(&[1u8; 32]);
        let tx = make_tx(&kp, 0, 10);

        assert!(pool.insert(tx.clone()).is_ok());
        assert_eq!(pool.insert(tx), Err(RejectReason::AlreadyKnown));
    }

    #[test]
    fn replace_by_fee() {
        let mut pool = Mempool::new(MempoolConfig::default());
        let kp = Keypair::from_seed(&[1u8; 32]);

        let tx1 = make_tx(&kp, 0, 10);
        let hash1 = tx1.hash();
        assert!(pool.insert(tx1).is_ok());

        // Same sender+nonce, higher gas price (>10% bump).
        let tx2 = make_tx(&kp, 0, 12);
        let hash2 = tx2.hash();
        assert!(pool.insert(tx2).is_ok());

        // Old tx should be gone, new tx present.
        assert!(!pool.contains(&hash1));
        assert!(pool.contains(&hash2));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn reject_insufficient_gas_bump() {
        let mut pool = Mempool::new(MempoolConfig::default());
        let kp = Keypair::from_seed(&[1u8; 32]);

        let tx1 = make_tx(&kp, 0, 10);
        assert!(pool.insert(tx1).is_ok());

        // Same sender+nonce with only ~5% bump (need >=10%).
        // Gas price 10 requires 10 + max(10*10/100, 1) = 11.
        // Use 10 + a different value field to get a different hash but same nonce.
        let to = Address::from_public_key(&[3u8; 32]); // different `to` = different hash
        let tx2 = Transaction::sign_transfer(&kp, to, 200, 0, 10, 21000, 1);
        let result = pool.insert(tx2);
        assert!(matches!(
            result,
            Err(RejectReason::InsufficientGasBump { .. })
        ));
    }

    #[test]
    fn evict_lowest_price_when_full() {
        let config = MempoolConfig {
            max_size: 3,
            ..MempoolConfig::default()
        };
        let mut pool = Mempool::new(config);

        // Insert 3 txs at different gas prices from different senders.
        for (i, price) in [(1u8, 5u64), (2, 10), (3, 15)] {
            let kp = Keypair::from_seed(&[i; 32]);
            assert!(pool.insert(make_tx(&kp, 0, price)).is_ok());
        }
        assert_eq!(pool.len(), 3);

        // Insert a 4th with higher price than the lowest (5).
        let kp4 = Keypair::from_seed(&[4u8; 32]);
        let tx4 = make_tx(&kp4, 0, 20);
        assert!(pool.insert(tx4).is_ok());
        assert_eq!(pool.len(), 3); // still 3 — lowest was evicted

        // The tx with gas_price=5 should be gone.
        let kp1 = Keypair::from_seed(&[1u8; 32]);
        let hash1 = make_tx(&kp1, 0, 5).hash();
        assert!(!pool.contains(&hash1));
    }

    #[test]
    fn reject_when_full_and_price_too_low() {
        let config = MempoolConfig {
            max_size: 2,
            ..MempoolConfig::default()
        };
        let mut pool = Mempool::new(config);

        let kp1 = Keypair::from_seed(&[1u8; 32]);
        let kp2 = Keypair::from_seed(&[2u8; 32]);
        assert!(pool.insert(make_tx(&kp1, 0, 10)).is_ok());
        assert!(pool.insert(make_tx(&kp2, 0, 20)).is_ok());

        // Try to insert with lower price than both.
        let kp3 = Keypair::from_seed(&[3u8; 32]);
        assert_eq!(
            pool.insert(make_tx(&kp3, 0, 5)),
            Err(RejectReason::PoolFull)
        );
    }

    #[test]
    fn pending_sorted_returns_highest_first() {
        let mut pool = Mempool::new(MempoolConfig::default());

        let kp1 = Keypair::from_seed(&[1u8; 32]);
        let kp2 = Keypair::from_seed(&[2u8; 32]);
        let kp3 = Keypair::from_seed(&[3u8; 32]);

        assert!(pool.insert(make_tx(&kp1, 0, 5)).is_ok());
        assert!(pool.insert(make_tx(&kp2, 0, 20)).is_ok());
        assert!(pool.insert(make_tx(&kp3, 0, 10)).is_ok());

        let sorted = pool.pending_sorted(10);
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].gas_price, 20);
        assert_eq!(sorted[1].gas_price, 10);
        assert_eq!(sorted[2].gas_price, 5);
    }

    #[test]
    fn remove_batch() {
        let mut pool = Mempool::new(MempoolConfig::default());
        let kp = Keypair::from_seed(&[1u8; 32]);

        let tx0 = make_tx(&kp, 0, 10);
        let tx1 = make_tx(&kp, 1, 10);
        let h0 = tx0.hash();
        let h1 = tx1.hash();

        assert!(pool.insert(tx0).is_ok());
        assert!(pool.insert(tx1).is_ok());
        assert_eq!(pool.len(), 2);

        pool.remove_batch(&[h0, h1]);
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn evict_expired() {
        let config = MempoolConfig {
            tx_expiry: Duration::from_millis(1),
            ..MempoolConfig::default()
        };
        let mut pool = Mempool::new(config);
        let kp = Keypair::from_seed(&[1u8; 32]);

        assert!(pool.insert(make_tx(&kp, 0, 10)).is_ok());
        std::thread::sleep(Duration::from_millis(10));

        let evicted = pool.evict_expired();
        assert_eq!(evicted, 1);
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn reject_oversized_transaction() {
        let mut pool = Mempool::new(MempoolConfig::default());
        let kp = Keypair::from_seed(&[1u8; 32]);
        let to = Address::from_public_key(&[2u8; 32]);
        let big = Transaction::sign_raw(
            &kp, Some(to), 0, 0, 1, 100_000,
            vec![0u8; karoowa_core::MAX_TX_BYTES + 1], 1,
        );
        match pool.insert(big) {
            Err(RejectReason::Invalid(msg)) => assert!(msg.contains("bytes"), "{msg}"),
            other => panic!("expected oversized rejection, got {other:?}"),
        }
        assert!(pool.is_empty());
    }
}

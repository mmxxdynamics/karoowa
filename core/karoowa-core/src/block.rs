//! Block and block header types.
//!
//! A [`Block`] contains a [`BlockHeader`] plus a list of transactions. The
//! header commits to the parent hash, state root, transaction root, receipt
//! root, and proposer. Block hashes are SHA3-256 of the bincode-serialized
//! header.

use karoowa_crypto::{sha3_256, Address, Hash, MerkleTree};
use serde::{Deserialize, Serialize};

use crate::transaction::Transaction;

/// Header for a block in the Karoowa chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    /// Hash of the parent block. `Hash::ZERO` for the genesis block.
    pub parent_hash: Hash,
    /// Merkle root of the account state trie after executing this block.
    pub state_root: Hash,
    /// Merkle root of the transactions in this block.
    pub tx_root: Hash,
    /// Merkle root of the receipts produced by executing this block's txs.
    pub receipt_root: Hash,
    /// Block height (0-indexed; genesis = 0).
    pub height: u64,
    /// Unix timestamp (seconds since epoch) when this block was proposed.
    pub timestamp: u64,
    /// Address of the validator that proposed this block.
    pub proposer: Address,
    /// Opaque consensus-engine-specific data (e.g. round number, vote set).
    pub consensus_data: Vec<u8>,
}

impl BlockHeader {
    /// Compute the block hash (SHA3-256 of bincode-serialized header).
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(self).expect("header serialization cannot fail");
        sha3_256(&bytes)
    }
}

/// A full block: header + transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// The block header.
    pub header: BlockHeader,
    /// Transactions included in this block, in execution order.
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Convenience: the block hash (delegates to `header.hash()`).
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Convenience: the block height.
    pub fn height(&self) -> u64 {
        self.header.height
    }

    /// Compute the expected transaction Merkle root from this block's txs.
    ///
    /// Call this to validate that `header.tx_root` matches the actual
    /// transactions in the block body.
    pub fn compute_tx_root(&self) -> Hash {
        if self.transactions.is_empty() {
            return Hash::ZERO;
        }
        let leaf_hashes: Vec<Hash> = self.transactions.iter().map(|tx| tx.hash()).collect();
        let tree = MerkleTree::from_leaves(&leaf_hashes);
        tree.root()
    }

    /// Validate that the header's `tx_root` matches the transactions.
    pub fn validate_tx_root(&self) -> bool {
        self.header.tx_root == self.compute_tx_root()
    }
}

/// Builder for constructing blocks (used by the consensus engine / proposer).
pub struct BlockBuilder {
    parent_hash: Hash,
    height: u64,
    timestamp: u64,
    proposer: Address,
    state_root: Hash,
    receipt_root: Hash,
    consensus_data: Vec<u8>,
    transactions: Vec<Transaction>,
}

impl BlockBuilder {
    /// Start building a new block.
    pub fn new(parent_hash: Hash, height: u64, timestamp: u64, proposer: Address) -> Self {
        BlockBuilder {
            parent_hash,
            height,
            timestamp,
            proposer,
            state_root: Hash::ZERO,
            receipt_root: Hash::ZERO,
            consensus_data: Vec::new(),
            transactions: Vec::new(),
        }
    }

    /// Set the state root (after executing all transactions).
    pub fn state_root(mut self, root: Hash) -> Self {
        self.state_root = root;
        self
    }

    /// Set the receipt root.
    pub fn receipt_root(mut self, root: Hash) -> Self {
        self.receipt_root = root;
        self
    }

    /// Set consensus-engine-specific data.
    pub fn consensus_data(mut self, data: Vec<u8>) -> Self {
        self.consensus_data = data;
        self
    }

    /// Set the transactions for this block.
    pub fn transactions(mut self, txs: Vec<Transaction>) -> Self {
        self.transactions = txs;
        self
    }

    /// Finalize and build the block, computing the tx_root automatically.
    pub fn build(self) -> Block {
        let tx_root = if self.transactions.is_empty() {
            Hash::ZERO
        } else {
            let leaf_hashes: Vec<Hash> = self.transactions.iter().map(|tx| tx.hash()).collect();
            MerkleTree::from_leaves(&leaf_hashes).root()
        };

        Block {
            header: BlockHeader {
                parent_hash: self.parent_hash,
                state_root: self.state_root,
                tx_root,
                receipt_root: self.receipt_root,
                height: self.height,
                timestamp: self.timestamp,
                proposer: self.proposer,
                consensus_data: self.consensus_data,
            },
            transactions: self.transactions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_crypto::Keypair;

    fn make_tx(kp: &Keypair, nonce: u64) -> Transaction {
        let to = Address::from_public_key(&[2u8; 32]);
        Transaction::sign_transfer(kp, to, 100, nonce, 1, 21000, 1)
    }

    fn make_block(txs: Vec<Transaction>) -> Block {
        let proposer = Address::from_public_key(&[99u8; 32]);
        BlockBuilder::new(Hash::ZERO, 1, 1700000000, proposer)
            .transactions(txs)
            .build()
    }

    #[test]
    fn empty_block_has_zero_tx_root() {
        let block = make_block(vec![]);
        assert_eq!(block.header.tx_root, Hash::ZERO);
        assert!(block.validate_tx_root());
    }

    #[test]
    fn block_with_txs_has_valid_tx_root() {
        let kp = Keypair::from_seed(&[1u8; 32]);
        let txs = vec![make_tx(&kp, 0), make_tx(&kp, 1), make_tx(&kp, 2)];
        let block = make_block(txs);
        assert_ne!(block.header.tx_root, Hash::ZERO);
        assert!(block.validate_tx_root());
    }

    #[test]
    fn tampered_tx_invalidates_root() {
        let kp = Keypair::from_seed(&[1u8; 32]);
        let txs = vec![make_tx(&kp, 0), make_tx(&kp, 1)];
        let mut block = make_block(txs);
        // Tamper with a transaction after the block was built
        block.transactions[0].value = 99999;
        assert!(!block.validate_tx_root());
    }

    #[test]
    fn block_hash_is_deterministic() {
        let kp = Keypair::from_seed(&[1u8; 32]);
        let txs1 = vec![make_tx(&kp, 0)];
        let txs2 = vec![make_tx(&kp, 0)];
        let block1 = make_block(txs1);
        let block2 = make_block(txs2);
        assert_eq!(block1.hash(), block2.hash());
    }

    #[test]
    fn different_blocks_have_different_hashes() {
        let kp = Keypair::from_seed(&[1u8; 32]);
        let block1 = make_block(vec![make_tx(&kp, 0)]);
        let block2 = make_block(vec![make_tx(&kp, 1)]);
        assert_ne!(block1.hash(), block2.hash());
    }

    #[test]
    fn genesis_block() {
        let proposer = Address::from_public_key(&[99u8; 32]);
        let genesis = BlockBuilder::new(Hash::ZERO, 0, 0, proposer).build();
        assert_eq!(genesis.height(), 0);
        assert_eq!(genesis.header.parent_hash, Hash::ZERO);
        assert_eq!(genesis.header.tx_root, Hash::ZERO);
    }

    #[test]
    fn block_builder_sets_all_fields() {
        let proposer = Address::from_public_key(&[99u8; 32]);
        let state_root = karoowa_crypto::sha3_256(b"state");
        let receipt_root = karoowa_crypto::sha3_256(b"receipts");
        let block = BlockBuilder::new(Hash::ZERO, 5, 1700000000, proposer)
            .state_root(state_root)
            .receipt_root(receipt_root)
            .consensus_data(vec![0x42])
            .build();

        assert_eq!(block.header.height, 5);
        assert_eq!(block.header.state_root, state_root);
        assert_eq!(block.header.receipt_root, receipt_root);
        assert_eq!(block.header.consensus_data, vec![0x42]);
    }

    #[test]
    fn serde_roundtrip() {
        let kp = Keypair::from_seed(&[1u8; 32]);
        let block = make_block(vec![make_tx(&kp, 0)]);
        let json = serde_json::to_string(&block).unwrap();
        let deserialized: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(block.hash(), deserialized.hash());
        assert!(deserialized.validate_tx_root());
    }
}

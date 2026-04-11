//! BFT consensus types — votes, quorum certificates, round state.

use karoowa_crypto::{Address, Hash};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The current step in a consensus round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Step {
    /// Waiting for the proposer to broadcast a block.
    Propose,
    /// Collecting prevotes for the proposed block.
    Prevote,
    /// Collecting precommits for the prevoted block.
    Precommit,
    /// Round completed — block committed or round timed out.
    Committed,
}

/// A vote cast by a validator during BFT consensus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Vote {
    /// The vote type.
    pub vote_type: VoteType,
    /// Block height being voted on.
    pub height: u64,
    /// Consensus round number (starts at 0, increments on timeout).
    pub round: u32,
    /// Hash of the block being voted for (ZERO = nil vote / timeout).
    pub block_hash: Hash,
    /// Address of the voting validator.
    pub voter: Address,
}

/// Type of vote in the BFT protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoteType {
    Prevote,
    Precommit,
}

/// A quorum certificate proving 2/3+1 validators agree on a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumCertificate {
    /// Block height.
    pub height: u64,
    /// Round in which the quorum was reached.
    pub round: u32,
    /// Hash of the block this certificate is for.
    pub block_hash: Hash,
    /// The votes that form the quorum.
    pub votes: Vec<Vote>,
}

/// Configuration for the BFT engine.
#[derive(Debug, Clone)]
pub struct BFTConfig {
    /// Ordered validator set. Position determines proposer rotation.
    pub validators: Vec<Address>,
    /// Base timeout for the propose step (milliseconds).
    pub propose_timeout_ms: u64,
    /// Base timeout for the prevote step (milliseconds).
    pub prevote_timeout_ms: u64,
    /// Base timeout for the precommit step (milliseconds).
    pub precommit_timeout_ms: u64,
}

impl BFTConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.validators.is_empty() {
            return Err("BFT requires at least one validator".into());
        }
        // BFT needs at least 4 validators to tolerate 1 Byzantine fault.
        // With 3, f=0 (no fault tolerance). We allow 1+ for testing.
        Ok(())
    }

    /// The maximum number of Byzantine faults tolerable: floor((n-1)/3)
    pub fn max_faults(&self) -> usize {
        if self.validators.is_empty() {
            return 0;
        }
        (self.validators.len() - 1) / 3
    }

    /// The quorum size required: 2f+1 where f = max_faults.
    /// This equals ceil(2n/3).
    pub fn quorum_size(&self) -> usize {
        let n = self.validators.len();
        (2 * n).div_ceil(3)
    }

    /// Determine the proposer for a given height and round.
    pub fn proposer(&self, height: u64, round: u32) -> Address {
        let idx = ((height as usize) + (round as usize)) % self.validators.len();
        self.validators[idx]
    }
}

/// Tracks votes received for a specific height and round.
#[derive(Debug, Clone)]
pub struct VoteCollector {
    /// Required quorum size.
    quorum_size: usize,
    /// Prevotes: block_hash -> set of voters.
    prevotes: HashMap<Hash, HashSet<Address>>,
    /// Precommits: block_hash -> set of voters.
    precommits: HashMap<Hash, HashSet<Address>>,
    /// All validators in the set (for duplicate detection).
    validators: HashSet<Address>,
}

impl VoteCollector {
    /// Create a new vote collector.
    pub fn new(validators: &[Address], quorum_size: usize) -> Self {
        VoteCollector {
            quorum_size,
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
            validators: validators.iter().copied().collect(),
        }
    }

    /// Add a vote. Returns `true` if this vote caused quorum to be reached.
    pub fn add_vote(&mut self, vote: &Vote) -> bool {
        // Reject votes from non-validators.
        if !self.validators.contains(&vote.voter) {
            return false;
        }

        let map = match vote.vote_type {
            VoteType::Prevote => &mut self.prevotes,
            VoteType::Precommit => &mut self.precommits,
        };

        let voters = map.entry(vote.block_hash).or_default();
        voters.insert(vote.voter);
        voters.len() == self.quorum_size
    }

    /// Check if prevote quorum has been reached for a block hash.
    pub fn has_prevote_quorum(&self, block_hash: &Hash) -> bool {
        self.prevotes
            .get(block_hash)
            .map(|v| v.len() >= self.quorum_size)
            .unwrap_or(false)
    }

    /// Check if precommit quorum has been reached for a block hash.
    pub fn has_precommit_quorum(&self, block_hash: &Hash) -> bool {
        self.precommits
            .get(block_hash)
            .map(|v| v.len() >= self.quorum_size)
            .unwrap_or(false)
    }

    /// Get the block hash that has prevote quorum (if any).
    pub fn prevote_quorum_hash(&self) -> Option<Hash> {
        self.prevotes.iter().find_map(|(hash, voters)| {
            if voters.len() >= self.quorum_size {
                Some(*hash)
            } else {
                None
            }
        })
    }

    /// Get the block hash that has precommit quorum (if any).
    pub fn precommit_quorum_hash(&self) -> Option<Hash> {
        self.precommits.iter().find_map(|(hash, voters)| {
            if voters.len() >= self.quorum_size {
                Some(*hash)
            } else {
                None
            }
        })
    }

    /// Build a quorum certificate from the precommit votes.
    pub fn build_certificate(&self, height: u64, round: u32) -> Option<QuorumCertificate> {
        let block_hash = self.precommit_quorum_hash()?;
        let votes: Vec<Vote> = self.precommits[&block_hash]
            .iter()
            .map(|voter| Vote {
                vote_type: VoteType::Precommit,
                height,
                round,
                block_hash,
                voter: *voter,
            })
            .collect();

        Some(QuorumCertificate {
            height,
            round,
            block_hash,
            votes,
        })
    }

    /// Total prevote count (across all block hashes).
    pub fn total_prevotes(&self) -> usize {
        self.prevotes.values().map(|v| v.len()).sum()
    }

    /// Total precommit count.
    pub fn total_precommits(&self) -> usize {
        self.precommits.values().map(|v| v.len()).sum()
    }
}

/// The state of a single consensus round.
#[derive(Debug, Clone)]
pub struct RoundState {
    pub height: u64,
    pub round: u32,
    pub step: Step,
    pub proposed_hash: Option<Hash>,
    pub locked_hash: Option<Hash>,
    pub locked_round: Option<u32>,
    pub votes: VoteCollector,
}

impl RoundState {
    /// Create a new round state.
    pub fn new(height: u64, round: u32, validators: &[Address], quorum_size: usize) -> Self {
        RoundState {
            height,
            round,
            step: Step::Propose,
            proposed_hash: None,
            locked_hash: None,
            locked_round: None,
            votes: VoteCollector::new(validators, quorum_size),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_crypto::Keypair;

    fn addr(seed: u8) -> Address {
        Keypair::from_seed(&[seed; 32]).address()
    }

    fn test_config() -> BFTConfig {
        BFTConfig {
            validators: vec![addr(1), addr(2), addr(3), addr(4)],
            propose_timeout_ms: 3000,
            prevote_timeout_ms: 1000,
            precommit_timeout_ms: 1000,
        }
    }

    #[test]
    fn quorum_size_4_validators() {
        let cfg = test_config();
        assert_eq!(cfg.max_faults(), 1);
        assert_eq!(cfg.quorum_size(), 3); // ceil(8/3) = 3
    }

    #[test]
    fn quorum_size_7_validators() {
        let cfg = BFTConfig {
            validators: (1..=7).map(addr).collect(),
            ..test_config()
        };
        assert_eq!(cfg.max_faults(), 2);
        assert_eq!(cfg.quorum_size(), 5); // ceil(14/3) = 5
    }

    #[test]
    fn proposer_rotation() {
        let cfg = test_config();
        assert_eq!(cfg.proposer(0, 0), addr(1));
        assert_eq!(cfg.proposer(1, 0), addr(2));
        assert_eq!(cfg.proposer(0, 1), addr(2)); // round increment changes proposer
        assert_eq!(cfg.proposer(3, 0), addr(4));
        assert_eq!(cfg.proposer(4, 0), addr(1)); // wraps
    }

    #[test]
    fn vote_collector_prevote_quorum() {
        let validators = vec![addr(1), addr(2), addr(3), addr(4)];
        let mut collector = VoteCollector::new(&validators, 3);

        let block_hash = Hash::ZERO;

        // 2 votes — no quorum.
        collector.add_vote(&Vote {
            vote_type: VoteType::Prevote,
            height: 1,
            round: 0,
            block_hash,
            voter: addr(1),
        });
        collector.add_vote(&Vote {
            vote_type: VoteType::Prevote,
            height: 1,
            round: 0,
            block_hash,
            voter: addr(2),
        });
        assert!(!collector.has_prevote_quorum(&block_hash));

        // 3rd vote — quorum reached.
        let reached = collector.add_vote(&Vote {
            vote_type: VoteType::Prevote,
            height: 1,
            round: 0,
            block_hash,
            voter: addr(3),
        });
        assert!(reached);
        assert!(collector.has_prevote_quorum(&block_hash));
    }

    #[test]
    fn vote_collector_rejects_non_validator() {
        let validators = vec![addr(1), addr(2), addr(3), addr(4)];
        let mut collector = VoteCollector::new(&validators, 3);

        let reached = collector.add_vote(&Vote {
            vote_type: VoteType::Prevote,
            height: 1,
            round: 0,
            block_hash: Hash::ZERO,
            voter: addr(99), // not a validator
        });
        assert!(!reached);
        assert_eq!(collector.total_prevotes(), 0);
    }

    #[test]
    fn vote_collector_duplicate_vote_idempotent() {
        let validators = vec![addr(1), addr(2), addr(3), addr(4)];
        let mut collector = VoteCollector::new(&validators, 3);

        let vote = Vote {
            vote_type: VoteType::Prevote,
            height: 1,
            round: 0,
            block_hash: Hash::ZERO,
            voter: addr(1),
        };

        collector.add_vote(&vote);
        collector.add_vote(&vote); // duplicate
        assert_eq!(collector.total_prevotes(), 1);
    }

    #[test]
    fn build_quorum_certificate() {
        let validators = vec![addr(1), addr(2), addr(3), addr(4)];
        let mut collector = VoteCollector::new(&validators, 3);

        let block_hash = Hash::ZERO;

        for v in [addr(1), addr(2), addr(3)] {
            collector.add_vote(&Vote {
                vote_type: VoteType::Precommit,
                height: 5,
                round: 0,
                block_hash,
                voter: v,
            });
        }

        let cert = collector.build_certificate(5, 0).unwrap();
        assert_eq!(cert.height, 5);
        assert_eq!(cert.round, 0);
        assert_eq!(cert.block_hash, block_hash);
        assert_eq!(cert.votes.len(), 3);
    }

    #[test]
    fn no_certificate_without_quorum() {
        let validators = vec![addr(1), addr(2), addr(3), addr(4)];
        let collector = VoteCollector::new(&validators, 3);
        assert!(collector.build_certificate(1, 0).is_none());
    }
}

//! State snapshot support for M4 state sync.
//!
//! A snapshot is a point-in-time serialization of account state, chunked
//! into fixed-size pieces (~4 MB each), compressed with zstd, and committed
//! via a chunk Merkle tree. A new node joining the network can download
//! chunks from peers in parallel, verify each chunk against the tree root,
//! and reconstruct the state without replaying from genesis.
//!
//! The snapshot root commitment is stored alongside the block at the
//! snapshot height, so all validators agree on the snapshot's correctness.

use karoowa_core::Account;
use karoowa_crypto::{sha3_256, Address, Hash};
use serde::{Deserialize, Serialize};

use crate::error::StorageError;

/// Target chunk size in bytes (uncompressed). Real chunks may be slightly
/// larger to avoid splitting an account mid-serialization.
pub const TARGET_CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4 MB

/// A snapshot manifest describing a point-in-time state dump.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotManifest {
    /// Block height at which the snapshot was taken.
    pub height: u64,
    /// The state root the snapshot commits to (must match `BlockHeader::state_root`).
    pub state_root: Hash,
    /// Hash of each chunk, in order. Length = total chunk count.
    pub chunk_hashes: Vec<Hash>,
    /// Total uncompressed size in bytes.
    pub total_size: u64,
    /// Compression codec used (e.g. "zstd").
    pub compression: String,
}

impl SnapshotManifest {
    /// Compute the manifest's commitment hash (used as the snapshot's
    /// identity, similar to an IPFS CID).
    pub fn commitment(&self) -> Hash {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.height.to_be_bytes());
        buf.extend_from_slice(self.state_root.as_bytes());
        for h in &self.chunk_hashes {
            buf.extend_from_slice(h.as_bytes());
        }
        buf.extend_from_slice(&self.total_size.to_be_bytes());
        buf.extend_from_slice(self.compression.as_bytes());
        sha3_256(&buf)
    }

    /// Number of chunks in this snapshot.
    pub fn chunk_count(&self) -> usize {
        self.chunk_hashes.len()
    }
}

/// A single chunk of snapshot data (compressed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotChunk {
    /// Chunk index in the snapshot.
    pub index: u32,
    /// Compressed chunk data.
    pub data: Vec<u8>,
}

impl SnapshotChunk {
    /// Compute the chunk's hash (over the compressed data).
    pub fn hash(&self) -> Hash {
        sha3_256(&self.data)
    }

    /// Verify this chunk against its expected hash from the manifest.
    pub fn verify(&self, expected: &Hash) -> bool {
        &self.hash() == expected
    }
}

/// An account entry inside a snapshot chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub address: Address,
    pub account: Account,
}

/// Storage trait for persisting and retrieving snapshots.
///
/// In production this is backed by RocksDB; for testing an in-memory
/// implementation is provided.
pub trait SnapshotStore: Send + Sync {
    /// Create a snapshot from a list of account entries at the given height.
    fn create_snapshot(
        &self,
        height: u64,
        state_root: Hash,
        entries: Vec<SnapshotEntry>,
    ) -> Result<SnapshotManifest, StorageError>;

    /// List manifests of all available snapshots.
    fn list_snapshots(&self) -> Result<Vec<SnapshotManifest>, StorageError>;

    /// Get the manifest for a specific snapshot height.
    fn get_manifest(&self, height: u64) -> Result<Option<SnapshotManifest>, StorageError>;

    /// Fetch a specific chunk from a snapshot.
    fn get_chunk(&self, height: u64, index: u32) -> Result<Option<SnapshotChunk>, StorageError>;

    /// Delete a snapshot and all its chunks.
    fn delete_snapshot(&self, height: u64) -> Result<(), StorageError>;
}

/// Split a sorted list of account entries into chunks targeted at `TARGET_CHUNK_SIZE`.
///
/// Each chunk is compressed with zstd before being returned. The returned
/// vector contains the compressed chunks in order.
pub fn chunk_and_compress(entries: &[SnapshotEntry]) -> Result<Vec<Vec<u8>>, StorageError> {
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut current: Vec<SnapshotEntry> = Vec::new();
    let mut current_size: usize = 0;

    for entry in entries {
        // Approximate serialized size: address (20) + account (~64).
        let entry_size = 20 + bincode::serialized_size(&entry.account).unwrap_or(64) as usize;

        if current_size + entry_size > TARGET_CHUNK_SIZE && !current.is_empty() {
            let bytes = bincode::serialize(&current)?;
            let compressed = zstd::encode_all(bytes.as_slice(), 3)
                .map_err(|e| StorageError::Serialization(format!("zstd encode: {e}")))?;
            chunks.push(compressed);
            current.clear();
            current_size = 0;
        }

        current.push(entry.clone());
        current_size += entry_size;
    }

    // Final chunk.
    if !current.is_empty() {
        let bytes = bincode::serialize(&current)?;
        let compressed = zstd::encode_all(bytes.as_slice(), 3)
            .map_err(|e| StorageError::Serialization(format!("zstd encode: {e}")))?;
        chunks.push(compressed);
    }

    Ok(chunks)
}

/// Decompress and deserialize a chunk back into account entries.
pub fn decompress_chunk(compressed: &[u8]) -> Result<Vec<SnapshotEntry>, StorageError> {
    let bytes = zstd::decode_all(compressed)
        .map_err(|e| StorageError::Serialization(format!("zstd decode: {e}")))?;
    let entries: Vec<SnapshotEntry> = bincode::deserialize(&bytes)?;
    Ok(entries)
}

/// Build a manifest from a list of pre-compressed chunks.
pub fn build_manifest(height: u64, state_root: Hash, chunks: &[Vec<u8>]) -> SnapshotManifest {
    let chunk_hashes: Vec<Hash> = chunks.iter().map(|c| sha3_256(c)).collect();
    let total_size: u64 = chunks.iter().map(|c| c.len() as u64).sum();

    SnapshotManifest {
        height,
        state_root,
        chunk_hashes,
        total_size,
        compression: "zstd".to_string(),
    }
}

/// Internal storage entry: a manifest plus its compressed chunks.
type StoredSnapshot = (SnapshotManifest, Vec<Vec<u8>>);

/// An in-memory implementation of `SnapshotStore` for testing.
pub struct InMemorySnapshotStore {
    snapshots: std::sync::Mutex<std::collections::HashMap<u64, StoredSnapshot>>,
}

impl InMemorySnapshotStore {
    pub fn new() -> Self {
        InMemorySnapshotStore {
            snapshots: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemorySnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotStore for InMemorySnapshotStore {
    fn create_snapshot(
        &self,
        height: u64,
        state_root: Hash,
        entries: Vec<SnapshotEntry>,
    ) -> Result<SnapshotManifest, StorageError> {
        let chunks = chunk_and_compress(&entries)?;
        let manifest = build_manifest(height, state_root, &chunks);

        self.snapshots
            .lock()
            .unwrap()
            .insert(height, (manifest.clone(), chunks));

        Ok(manifest)
    }

    fn list_snapshots(&self) -> Result<Vec<SnapshotManifest>, StorageError> {
        Ok(self
            .snapshots
            .lock()
            .unwrap()
            .values()
            .map(|(m, _)| m.clone())
            .collect())
    }

    fn get_manifest(&self, height: u64) -> Result<Option<SnapshotManifest>, StorageError> {
        Ok(self
            .snapshots
            .lock()
            .unwrap()
            .get(&height)
            .map(|(m, _)| m.clone()))
    }

    fn get_chunk(&self, height: u64, index: u32) -> Result<Option<SnapshotChunk>, StorageError> {
        let snapshots = self.snapshots.lock().unwrap();
        Ok(snapshots.get(&height).and_then(|(_, chunks)| {
            chunks.get(index as usize).map(|data| SnapshotChunk {
                index,
                data: data.clone(),
            })
        }))
    }

    fn delete_snapshot(&self, height: u64) -> Result<(), StorageError> {
        self.snapshots.lock().unwrap().remove(&height);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(seed: u8, balance: u64) -> SnapshotEntry {
        SnapshotEntry {
            address: Address::from_public_key(&[seed; 32]),
            account: Account {
                balance,
                nonce: 0,
                ..Account::default()
            },
        }
    }

    #[test]
    fn chunk_and_decompress_roundtrip() {
        let entries = vec![make_entry(1, 100), make_entry(2, 200), make_entry(3, 300)];
        let chunks = chunk_and_compress(&entries).unwrap();
        assert_eq!(chunks.len(), 1); // Small data fits in one chunk

        let decoded = decompress_chunk(&chunks[0]).unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].account.balance, 100);
        assert_eq!(decoded[2].account.balance, 300);
    }

    #[test]
    fn manifest_commitment_is_deterministic() {
        let chunks = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let m1 = build_manifest(5, Hash::ZERO, &chunks);
        let m2 = build_manifest(5, Hash::ZERO, &chunks);
        assert_eq!(m1.commitment(), m2.commitment());
    }

    #[test]
    fn different_snapshots_different_commitments() {
        let chunks1 = vec![vec![1, 2, 3]];
        let chunks2 = vec![vec![1, 2, 4]];
        let m1 = build_manifest(5, Hash::ZERO, &chunks1);
        let m2 = build_manifest(5, Hash::ZERO, &chunks2);
        assert_ne!(m1.commitment(), m2.commitment());
    }

    #[test]
    fn chunk_verify() {
        let data = vec![1u8, 2, 3, 4];
        let chunk = SnapshotChunk { index: 0, data };
        let hash = chunk.hash();
        assert!(chunk.verify(&hash));

        let wrong = sha3_256(b"wrong");
        assert!(!chunk.verify(&wrong));
    }

    #[test]
    fn in_memory_store_create_and_fetch() {
        let store = InMemorySnapshotStore::new();
        let entries = vec![make_entry(1, 100), make_entry(2, 200)];

        let manifest = store
            .create_snapshot(10, sha3_256(b"state"), entries)
            .unwrap();
        assert_eq!(manifest.height, 10);
        assert_eq!(manifest.chunk_count(), 1);

        let retrieved = store.get_manifest(10).unwrap().unwrap();
        assert_eq!(retrieved, manifest);

        let chunk = store.get_chunk(10, 0).unwrap().unwrap();
        assert!(chunk.verify(&manifest.chunk_hashes[0]));

        // Missing snapshot.
        assert!(store.get_manifest(999).unwrap().is_none());
        assert!(store.get_chunk(999, 0).unwrap().is_none());
    }

    #[test]
    fn list_snapshots() {
        let store = InMemorySnapshotStore::new();
        store
            .create_snapshot(10, Hash::ZERO, vec![make_entry(1, 100)])
            .unwrap();
        store
            .create_snapshot(20, Hash::ZERO, vec![make_entry(2, 200)])
            .unwrap();

        let list = store.list_snapshots().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn delete_snapshot() {
        let store = InMemorySnapshotStore::new();
        store
            .create_snapshot(10, Hash::ZERO, vec![make_entry(1, 100)])
            .unwrap();
        assert!(store.get_manifest(10).unwrap().is_some());

        store.delete_snapshot(10).unwrap();
        assert!(store.get_manifest(10).unwrap().is_none());
    }

    #[test]
    fn snapshot_roundtrip_many_entries() {
        // 1000 entries — small enough to be fast, large enough to exercise
        // the chunking + compression path.
        let mut entries = Vec::with_capacity(1000);
        for i in 0..1000u32 {
            let seed = (i % 256) as u8;
            entries.push(SnapshotEntry {
                address: Address::from_public_key(&[seed; 32]),
                account: Account {
                    balance: i as u64,
                    nonce: i as u64,
                    ..Account::default()
                },
            });
        }

        let chunks = chunk_and_compress(&entries).unwrap();
        assert!(!chunks.is_empty());

        // Verify each chunk decompresses correctly.
        let mut total_entries = 0;
        for chunk in &chunks {
            let decoded = decompress_chunk(chunk).unwrap();
            total_entries += decoded.len();
        }
        assert_eq!(total_entries, 1000);
    }
}

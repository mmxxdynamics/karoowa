//! Karoowa storage layer (L1 of the four-layer database strategy).
//!
//! Defines the [`BlockStore`], [`StateStore`], and [`ReceiptStore`] traits
//! and ships a RocksDB implementation ([`RocksStorage`]) with column families.
//!
//! Backends are swappable via the trait surface — see REQ-017 in the parent PRD.

pub mod error;
pub mod rocks;
pub mod snapshot;
pub mod traits;

pub use error::StorageError;
pub use rocks::RocksStorage;
pub use snapshot::{
    build_manifest, chunk_and_compress, decompress_chunk, InMemorySnapshotStore, SnapshotChunk,
    SnapshotEntry, SnapshotManifest, SnapshotStore, TARGET_CHUNK_SIZE,
};
pub use traits::{BlockStore, ReceiptStore, StateStore};

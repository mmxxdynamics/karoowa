//! Karoowa storage layer (L1 of the four-layer database strategy).
//!
//! Defines the [`BlockStore`], [`StateStore`], and [`ReceiptStore`] traits
//! and ships a RocksDB implementation ([`RocksStorage`]) with column families.
//!
//! Backends are swappable via the trait surface — see REQ-017 in the parent PRD.

pub mod error;
pub mod rocks;
pub mod traits;

pub use error::StorageError;
pub use rocks::RocksStorage;
pub use traits::{BlockStore, ReceiptStore, StateStore};

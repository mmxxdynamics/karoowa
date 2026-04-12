//! Karoowa cross-chain bridge primitives.
//!
//! Provides the building blocks for transferring tokens between Karoowa
//! chains via the lock-and-mint model:
//!
//! - [`BridgePacket`] — a cross-chain transfer message
//! - [`BridgeChannel`] — a bidirectional pipe between two chains with a
//!   four-step handshake
//! - [`EscrowStore`] — trait for the on-chain escrow + wrapped balance store
//! - [`BridgeRelayer`] — coordinator that uses Merkle proofs to relay packets
//!   verifiably between chains
//!
//! # Verification
//!
//! Packets are committed on the source chain by storing a hash in the state
//! trie. The relayer fetches a Merkle proof of that commitment and submits
//! it to the destination chain along with the packet. The destination chain
//! verifies the proof against the source chain's `state_root` (obtained via
//! the light client crate, which it trusts after running the BFT consensus
//! verification chain from a known checkpoint).
//!
//! # Replay protection
//!
//! Each processed packet's hash is stored in a per-relayer set. Duplicate
//! deliveries are rejected with [`BridgeError::DuplicatePacket`].
//!
//! # Status
//!
//! This is the **minimum viable bridge** delivering Phase 5.0 of the M5
//! plan. Full IBC interoperability via `ibc-rs` integration is tracked as
//! Phase 5.0.b. The Karoowa-native protocol is intentionally simpler but
//! shares the same conceptual model (clients, channels, packets,
//! commitments) so the IBC migration can layer cleanly on top.

pub mod channel;
pub mod error;
pub mod escrow;
pub mod packet;
pub mod relayer;

pub use channel::{BridgeChannel, ChannelState};
pub use error::BridgeError;
pub use escrow::{BalanceEntry, EscrowStore, InMemoryEscrow};
pub use packet::{Acknowledgement, BridgePacket, PacketProof};
pub use relayer::BridgeRelayer;

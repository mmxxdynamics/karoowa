//! Karoowa light client.
//!
//! A light client stores block headers and the current validator set
//! without running a full node. It can verify account state by checking
//! a Merkle proof from a full node against the `state_root` in the
//! relevant block header.
//!
//! # Trust model
//!
//! The light client is initialized from a **trusted checkpoint** — a
//! recent block header from a known source (genesis or a node the user
//! trusts). Each subsequent header is verified by checking that:
//!
//! 1. The proposer is in the active validator set.
//! 2. The height is exactly `head.height + 1`.
//! 3. The `parent_hash` matches the current head's hash.
//!
//! Validator set rotations are tracked externally via
//! [`LightClient::update_validator_set`] — the light client does not
//! infer them from header contents.
//!
//! # Quick start
//!
//! ```ignore
//! use karoowa_light::{LightClient, ValidatorSetView};
//! use karoowa_core::BlockHeader;
//!
//! let validators = ValidatorSetView::new(vec![addr_a, addr_b, addr_c], 0);
//! let mut client = LightClient::new(genesis_header, validators)?;
//!
//! // Sync new headers from a full node.
//! client.append_header(header_1)?;
//! client.append_header(header_2)?;
//!
//! // Verify an account balance via a Merkle proof.
//! let value = client.verify_and_get(2, &merkle_proof)?;
//! ```

pub mod client;
pub mod error;
pub mod validator_set;

pub use client::LightClient;
pub use error::LightClientError;
pub use validator_set::ValidatorSetView;

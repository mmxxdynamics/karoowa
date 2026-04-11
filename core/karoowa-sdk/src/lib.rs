//! Karoowa client SDK.
//!
//! Provides [`NodeClient`] (async HTTP wrapper around the JSON-RPC surface)
//! and [`Wallet`] (key management + transaction signing) for dApp developers.
//!
//! # Quick start
//!
//! ```no_run
//! use karoowa_sdk::{NodeClient, Wallet, TransferBuilder};
//! use karoowa_crypto::Address;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = NodeClient::new("http://localhost:8545");
//! let chain_id = client.chain_id().await?;
//!
//! let wallet = Wallet::generate(chain_id);
//! let to = Address::from_public_key(&[2u8; 32]);
//!
//! let tx = TransferBuilder::new()
//!     .to(to)
//!     .value(100)
//!     .nonce(0)
//!     .sign(&wallet);
//!
//! let hex = Wallet::encode_transaction(&tx)?;
//! let tx_hash = client.send_raw_transaction(&hex).await?;
//! # Ok(())
//! # }
//! ```

pub mod builder;
pub mod client;
pub mod error;
pub mod wallet;

pub use builder::{ContractCallBuilder, TransferBuilder};
pub use client::NodeClient;
pub use error::SdkError;
pub use wallet::Wallet;

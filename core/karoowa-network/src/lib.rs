//! Karoowa P2P networking layer.
//!
//! Wraps libp2p with Gossipsub (block/tx broadcast) and Kademlia (peer
//! discovery) over TCP + Noise + Yamux. The [`Network`] struct builds the
//! swarm and spawns an event loop; interaction happens through the returned
//! [`NetworkHandle`].
//!
//! # Quick start
//!
//! ```no_run
//! use karoowa_network::{Network, NetworkConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = NetworkConfig::default();
//! let handle = Network::start(config).await?;
//!
//! // Subscribe to incoming blocks.
//! let mut blocks = handle.subscribe_blocks();
//!
//! // Check peer count.
//! let count = handle.peer_count();
//! # Ok(())
//! # }
//! ```

pub mod behaviour;
pub mod config;
pub mod error;
pub mod light_client;
pub mod state_sync;
pub mod swarm;

pub use config::NetworkConfig;
pub use error::NetworkError;
pub use light_client::{LightClientProvider, LightClientRequest, LightClientResponse};
pub use state_sync::{SnapshotProvider, SnapshotRequest, SnapshotResponse};
pub use swarm::{Network, NetworkHandle};

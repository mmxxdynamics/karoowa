//! Network configuration.

use libp2p::Multiaddr;
use std::time::Duration;

/// Configuration for the Karoowa P2P network.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Address to listen on (e.g. `/ip4/0.0.0.0/tcp/30303`).
    pub listen_addr: Multiaddr,

    /// Bootnode addresses to connect to on startup.
    pub bootnodes: Vec<Multiaddr>,

    /// Optional fixed seed for deterministic PeerId generation.
    /// If `None`, a random identity is generated.
    pub keypair_seed: Option<[u8; 32]>,

    /// Gossipsub heartbeat interval.
    pub gossipsub_heartbeat: Duration,

    /// Kademlia query timeout.
    pub kademlia_query_timeout: Duration,

    /// Target number of mesh peers for Gossipsub.
    pub mesh_n: usize,

    /// Lower bound for mesh peers.
    pub mesh_n_low: usize,

    /// Upper bound for mesh peers.
    pub mesh_n_high: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            listen_addr: "/ip4/0.0.0.0/tcp/30303".parse().unwrap(),
            bootnodes: Vec::new(),
            keypair_seed: None,
            gossipsub_heartbeat: Duration::from_secs(1),
            kademlia_query_timeout: Duration::from_secs(60),
            mesh_n: 6,
            mesh_n_low: 4,
            mesh_n_high: 12,
        }
    }
}

impl NetworkConfig {
    /// Create a config for testing — listens on a random OS-assigned port.
    #[cfg(test)]
    pub fn test_config(seed: u8) -> Self {
        NetworkConfig {
            listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
            bootnodes: Vec::new(),
            keypair_seed: Some([seed; 32]),
            gossipsub_heartbeat: Duration::from_millis(500),
            kademlia_query_timeout: Duration::from_secs(10),
            mesh_n: 2,
            mesh_n_low: 1,
            mesh_n_high: 4,
        }
    }
}

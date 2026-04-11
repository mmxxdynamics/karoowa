//! Shared application state for all API handlers.

use karoowa_consensus::MempoolHandle;
use karoowa_network::NetworkHandle;
use karoowa_storage::RocksStorage;
use std::sync::Arc;

/// Application state shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    /// Chain ID for this network.
    pub chain_id: u64,
    /// Storage backend.
    pub storage: Arc<RocksStorage>,
    /// Network handle for broadcasting and peer info.
    pub network: NetworkHandle,
    /// Transaction mempool.
    pub mempool: MempoolHandle,
}

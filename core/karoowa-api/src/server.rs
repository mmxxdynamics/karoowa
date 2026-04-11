//! API server — builds the Axum router and starts listening.

use axum::routing::{get, post};
use axum::Router;
use karoowa_consensus::{Mempool, MempoolConfig, MempoolHandle};
use karoowa_network::NetworkHandle;
use karoowa_storage::RocksStorage;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use crate::error::ApiError;
use crate::state::AppState;
use crate::{health, rest, rpc, ws};

/// Configuration for the API server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to (e.g. `0.0.0.0:8545`).
    pub bind_addr: SocketAddr,
    /// Chain ID for JSON-RPC responses.
    pub chain_id: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_addr: SocketAddr::from(([0, 0, 0, 0], 8545)),
            chain_id: 1,
        }
    }
}

/// Build the Axum router with all route groups.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // JSON-RPC
        .route("/rpc", post(rpc::rpc_handler))
        // REST
        .route("/api/v1/status", get(rest::status))
        .route("/api/v1/blocks/{height}", get(rest::block_by_height))
        .route("/api/v1/blocks/hash/{hash}", get(rest::block_by_hash))
        .route("/api/v1/tx/{hash}", get(rest::transaction_by_hash))
        .route("/api/v1/account/{address}", get(rest::account))
        // Health & Metrics
        .route("/health", get(health::health))
        .route("/metrics", get(health::metrics_handler))
        // WebSocket
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}

/// Start the API server. Returns the bound address, a mempool handle
/// (for the block producer to use), and the server task handle.
pub async fn start_server(
    config: ServerConfig,
    storage: Arc<RocksStorage>,
    network: NetworkHandle,
) -> Result<(SocketAddr, MempoolHandle, tokio::task::JoinHandle<()>), ApiError> {
    let mempool = MempoolHandle::new(Mempool::new(MempoolConfig::default()));

    let state = AppState {
        chain_id: config.chain_id,
        storage,
        network,
        mempool: mempool.clone(),
    };

    let app = build_router(state);

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .map_err(|e| ApiError::Internal(format!("bind failed: {e}")))?;

    let addr = listener
        .local_addr()
        .map_err(|e| ApiError::Internal(format!("local_addr: {e}")))?;

    info!(addr = %addr, "API server listening");

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    Ok((addr, mempool, handle))
}

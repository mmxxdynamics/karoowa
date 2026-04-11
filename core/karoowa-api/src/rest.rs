//! REST API handlers — `/api/v1/*`

use axum::extract::{Path, State};
use axum::Json;
use karoowa_crypto::{Address, Hash};
use karoowa_storage::{BlockStore, ReceiptStore, StateStore};
use serde_json::Value;

use crate::error::ApiError;
use crate::state::AppState;

/// GET /api/v1/status
pub async fn status(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let head_height = state.storage.head_height()?.unwrap_or(0);
    let peer_count = state.network.peer_count();
    Ok(Json(serde_json::json!({
        "chain_id": state.chain_id,
        "block_height": head_height,
        "peer_count": peer_count,
        "syncing": false,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

/// GET /api/v1/blocks/:height
pub async fn block_by_height(
    State(state): State<AppState>,
    Path(height): Path<u64>,
) -> Result<Json<Value>, ApiError> {
    match state.storage.get_block_by_height(height)? {
        Some(block) => Ok(Json(serde_json::to_value(&block).unwrap_or(Value::Null))),
        None => Err(ApiError::NotFound(format!("block at height {height}"))),
    }
}

/// GET /api/v1/blocks/hash/:hash
pub async fn block_by_hash(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let hash: Hash = hash_str
        .parse()
        .map_err(|e| ApiError::InvalidParams(format!("invalid hash: {e}")))?;
    match state.storage.get_block_by_hash(&hash)? {
        Some(block) => Ok(Json(serde_json::to_value(&block).unwrap_or(Value::Null))),
        None => Err(ApiError::NotFound(format!("block {hash_str}"))),
    }
}

/// GET /api/v1/tx/:hash
pub async fn transaction_by_hash(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let hash: Hash = hash_str
        .parse()
        .map_err(|e| ApiError::InvalidParams(format!("invalid hash: {e}")))?;
    match state.storage.get_receipt_by_tx_hash(&hash)? {
        Some(receipt) => Ok(Json(serde_json::to_value(&receipt).unwrap_or(Value::Null))),
        None => Err(ApiError::NotFound(format!("transaction {hash_str}"))),
    }
}

/// GET /api/v1/account/:address
pub async fn account(
    State(state): State<AppState>,
    Path(addr_str): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let addr: Address = addr_str
        .parse()
        .map_err(|e| ApiError::InvalidParams(format!("invalid address: {e}")))?;
    match state.storage.get_account(&addr)? {
        Some(account) => Ok(Json(serde_json::to_value(&account).unwrap_or(Value::Null))),
        None => Ok(Json(serde_json::json!({
            "address": addr_str,
            "balance": 0,
            "nonce": 0,
        }))),
    }
}

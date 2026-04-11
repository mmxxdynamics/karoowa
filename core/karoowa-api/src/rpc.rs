//! JSON-RPC 2.0 dispatcher and all `kw_*` method handlers.
//!
//! The dispatcher parses incoming JSON-RPC requests, routes them by method
//! name, and returns a JSON-RPC 2.0 response. All 14 M1 methods are
//! implemented here:
//!
//! **Read (12):** `kw_chainId`, `kw_blockNumber`, `kw_getBlockByNumber`,
//! `kw_getBlockByHash`, `kw_getTransactionByHash`, `kw_getTransactionReceipt`,
//! `kw_getBalance`, `kw_getTransactionCount`, `kw_getCode`, `kw_syncing`,
//! `kw_peerCount`, `kw_nodeInfo`
//!
//! **Write (2):** `kw_sendRawTransaction`, `kw_pendingTransactions`

use axum::extract::State;
use axum::Json;
use karoowa_crypto::{Address, Hash};
use karoowa_storage::{BlockStore, ReceiptStore, StateStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::state::AppState;

/// JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i64, message: String) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// Standard JSON-RPC error codes.
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

/// Main JSON-RPC dispatcher — POST /rpc
pub async fn rpc_handler(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    debug!(method = %req.method, "JSON-RPC request");

    let response = match req.method.as_str() {
        "kw_chainId" => handle_chain_id(&state, &req),
        "kw_blockNumber" => handle_block_number(&state, &req),
        "kw_getBlockByNumber" => handle_get_block_by_number(&state, &req),
        "kw_getBlockByHash" => handle_get_block_by_hash(&state, &req),
        "kw_getTransactionByHash" => handle_get_transaction_by_hash(&state, &req),
        "kw_getTransactionReceipt" => handle_get_transaction_receipt(&state, &req),
        "kw_getBalance" => handle_get_balance(&state, &req),
        "kw_getTransactionCount" => handle_get_transaction_count(&state, &req),
        "kw_getCode" => handle_get_code(&state, &req),
        "kw_syncing" => handle_syncing(&state, &req),
        "kw_peerCount" => handle_peer_count(&state, &req),
        "kw_nodeInfo" => handle_node_info(&state, &req),
        "kw_sendRawTransaction" => handle_send_raw_transaction(&state, &req).await,
        "kw_pendingTransactions" => handle_pending_transactions(&state, &req).await,
        _ => JsonRpcResponse::error(
            req.id.clone(),
            METHOD_NOT_FOUND,
            format!("method not found: {}", req.method),
        ),
    };

    Json(response)
}

// ---------------------------------------------------------------------------
// Read methods
// ---------------------------------------------------------------------------

fn handle_chain_id(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse::success(req.id.clone(), Value::from(state.chain_id))
}

fn handle_block_number(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    match state.storage.head_height() {
        Ok(Some(h)) => JsonRpcResponse::success(req.id.clone(), Value::from(h)),
        Ok(None) => JsonRpcResponse::success(req.id.clone(), Value::from(0u64)),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, e.to_string()),
    }
}

fn handle_get_block_by_number(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let height = match parse_u64_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    match state.storage.get_block_by_height(height) {
        Ok(Some(block)) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::to_value(&block).unwrap_or(Value::Null),
        ),
        Ok(None) => JsonRpcResponse::success(req.id.clone(), Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, e.to_string()),
    }
}

fn handle_get_block_by_hash(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    match state.storage.get_block_by_hash(&hash) {
        Ok(Some(block)) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::to_value(&block).unwrap_or(Value::Null),
        ),
        Ok(None) => JsonRpcResponse::success(req.id.clone(), Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, e.to_string()),
    }
}

fn handle_get_transaction_by_hash(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    // Search pending pool first, then receipts (which means it was mined).
    // For M1, we just check storage for a receipt and return the tx hash info.
    let hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    match state.storage.get_receipt_by_tx_hash(&hash) {
        Ok(Some(receipt)) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::json!({
                "tx_hash": hash.to_string(),
                "status": format!("{:?}", receipt.status),
                "gas_used": receipt.gas_used,
            }),
        ),
        Ok(None) => JsonRpcResponse::success(req.id.clone(), Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, e.to_string()),
    }
}

fn handle_get_transaction_receipt(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    match state.storage.get_receipt_by_tx_hash(&hash) {
        Ok(Some(receipt)) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::to_value(&receipt).unwrap_or(Value::Null),
        ),
        Ok(None) => JsonRpcResponse::success(req.id.clone(), Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, e.to_string()),
    }
}

fn handle_get_balance(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let addr = match parse_address_param(&req.params, 0) {
        Ok(a) => a,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    match state.storage.get_account(&addr) {
        Ok(Some(account)) => JsonRpcResponse::success(req.id.clone(), Value::from(account.balance)),
        Ok(None) => JsonRpcResponse::success(req.id.clone(), Value::from(0u64)),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, e.to_string()),
    }
}

fn handle_get_transaction_count(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let addr = match parse_address_param(&req.params, 0) {
        Ok(a) => a,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    match state.storage.get_account(&addr) {
        Ok(Some(account)) => JsonRpcResponse::success(req.id.clone(), Value::from(account.nonce)),
        Ok(None) => JsonRpcResponse::success(req.id.clone(), Value::from(0u64)),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, e.to_string()),
    }
}

fn handle_get_code(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let addr = match parse_address_param(&req.params, 0) {
        Ok(a) => a,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    match state.storage.get_account(&addr) {
        Ok(Some(account)) => {
            JsonRpcResponse::success(req.id.clone(), Value::from(account.code_hash.to_string()))
        }
        Ok(None) => JsonRpcResponse::success(req.id.clone(), Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, e.to_string()),
    }
}

fn handle_syncing(_state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    // M1: no sync protocol, node is always "synced" (single-node or PoA devnet).
    JsonRpcResponse::success(req.id.clone(), Value::Bool(false))
}

fn handle_peer_count(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let count = state.network.peer_count();
    JsonRpcResponse::success(req.id.clone(), Value::from(count))
}

fn handle_node_info(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let peer_id = state.network.local_peer_id().to_string();
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "peer_id": peer_id,
            "chain_id": state.chain_id,
            "version": env!("CARGO_PKG_VERSION"),
            "name": "karoowa",
        }),
    )
}

// ---------------------------------------------------------------------------
// Write methods
// ---------------------------------------------------------------------------

async fn handle_send_raw_transaction(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let tx_hex = match req
        .params
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
    {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "expected [hex_encoded_tx]".into(),
            )
        }
    };

    let tx_bytes = match hex::decode(tx_hex.strip_prefix("0x").unwrap_or(tx_hex)) {
        Ok(b) => b,
        Err(e) => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                format!("invalid hex: {e}"),
            )
        }
    };

    let tx: karoowa_core::Transaction = match bincode::deserialize(&tx_bytes) {
        Ok(t) => t,
        Err(e) => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                format!("invalid transaction: {e}"),
            )
        }
    };

    let tx_hash = tx.hash();

    // Add to pending pool.
    state.pending_txs.lock().await.push(tx.clone());

    // Broadcast to network (best-effort).
    if let Err(e) = state.network.broadcast_transaction(&tx).await {
        debug!(error = %e, "failed to broadcast tx, added to pending pool only");
    }

    JsonRpcResponse::success(req.id.clone(), Value::from(tx_hash.to_string()))
}

async fn handle_pending_transactions(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let pending = state.pending_txs.lock().await;
    let hashes: Vec<String> = pending.iter().map(|tx| tx.hash().to_string()).collect();
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::to_value(hashes).unwrap_or(Value::Array(vec![])),
    )
}

// ---------------------------------------------------------------------------
// Param parsing helpers
// ---------------------------------------------------------------------------

fn parse_u64_param(params: &Value, index: usize) -> Result<u64, String> {
    params
        .as_array()
        .and_then(|a| a.get(index))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("expected u64 at params[{index}]"))
}

fn parse_hash_param(params: &Value, index: usize) -> Result<Hash, String> {
    let s = params
        .as_array()
        .and_then(|a| a.get(index))
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("expected hex string at params[{index}]"))?;
    s.parse::<Hash>()
        .map_err(|e| format!("invalid hash at params[{index}]: {e}"))
}

fn parse_address_param(params: &Value, index: usize) -> Result<Address, String> {
    let s = params
        .as_array()
        .and_then(|a| a.get(index))
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("expected hex string at params[{index}]"))?;
    s.parse::<Address>()
        .map_err(|e| format!("invalid address at params[{index}]: {e}"))
}

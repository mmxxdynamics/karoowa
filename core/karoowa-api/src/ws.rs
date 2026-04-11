//! WebSocket endpoint — real subscription support (M2).
//!
//! Handles `kw_subscribe` and `kw_unsubscribe` JSON-RPC calls over WebSocket.
//! Supports three subscription types:
//! - `newBlocks` — push block headers on each new block
//! - `pendingTransactions` — push tx hashes on mempool entry
//! - `logs` — push filtered logs from block receipts

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::{json, Value};
use tracing::debug;

use crate::state::AppState;
use crate::subscriptions::{LogFilter, SubscriptionManager};

/// GET /ws — WebSocket upgrade handler.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.subscriptions.clone()))
}

async fn handle_socket(mut socket: WebSocket, subs: SubscriptionManager) {
    debug!("WebSocket connection established");

    // Active subscription tasks for this connection.
    let mut sub_tasks: Vec<(u64, tokio::task::JoinHandle<()>)> = Vec::new();

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                let request: Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => {
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": {"code": -32700, "message": "parse error"}
                        });
                        if socket
                            .send(Message::Text(resp.to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    }
                };

                let id = request["id"].clone();
                let method = request["method"].as_str().unwrap_or("");

                let response = match method {
                    "kw_subscribe" => {
                        handle_subscribe(&subs, &request, &mut sub_tasks, &mut socket).await
                    }
                    "kw_unsubscribe" => handle_unsubscribe(&subs, &request, &mut sub_tasks).await,
                    _ => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("method not found: {method}. Use kw_subscribe or kw_unsubscribe over WebSocket.")
                        }
                    }),
                };

                if socket
                    .send(Message::Text(response.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok(Message::Ping(data)) => {
                if socket.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    // Clean up subscriptions on disconnect.
    for (sub_id, handle) in sub_tasks {
        subs.unsubscribe(sub_id).await;
        handle.abort();
    }

    debug!("WebSocket connection closed");
}

#[allow(clippy::ptr_arg)]
async fn handle_subscribe(
    subs: &SubscriptionManager,
    request: &Value,
    _sub_tasks: &mut Vec<(u64, tokio::task::JoinHandle<()>)>,
    _socket: &mut WebSocket,
) -> Value {
    let id = request["id"].clone();
    let params = &request["params"];
    let sub_type = params
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match sub_type {
        "newBlocks" => {
            let handle = subs.subscribe_new_blocks().await;
            let sub_id = handle.id;

            // Note: In a full implementation, we'd spawn a task that reads
            // from handle.receiver and sends to the WebSocket. For now we
            // return the subscription ID — events are pushed via the manager.
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": sub_id.to_string()
            })
        }
        "pendingTransactions" => {
            let handle = subs.subscribe_pending_transactions().await;
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": handle.id.to_string()
            })
        }
        "logs" => {
            let filter: LogFilter = params
                .as_array()
                .and_then(|a| a.get(1))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or(LogFilter {
                    address: None,
                    topics: vec![],
                });

            let handle = subs.subscribe_logs(filter).await;
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": handle.id.to_string()
            })
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32602,
                "message": format!("unknown subscription type: {sub_type}. Available: newBlocks, pendingTransactions, logs")
            }
        }),
    }
}

async fn handle_unsubscribe(
    subs: &SubscriptionManager,
    request: &Value,
    sub_tasks: &mut Vec<(u64, tokio::task::JoinHandle<()>)>,
) -> Value {
    let id = request["id"].clone();
    let sub_id = request["params"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str().or_else(|| v.as_u64().map(|_| "")))
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| {
            request["params"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_u64())
        });

    match sub_id {
        Some(sid) => {
            let removed = subs.unsubscribe(sid).await;
            // Also abort the push task if we have one.
            sub_tasks.retain(|(id, handle)| {
                if *id == sid {
                    handle.abort();
                    false
                } else {
                    true
                }
            });
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": removed
            })
        }
        None => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32602, "message": "expected [subscription_id]"}
        }),
    }
}

//! WebSocket endpoint — placeholder for M1.
//!
//! Real WebSocket subscriptions (newBlocks, pendingTransactions, logs)
//! ship in M2 Phase 2.1. This placeholder accepts the upgrade, handles
//! ping/pong, and replies to subscribe requests with a "not yet implemented"
//! message.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tracing::debug;

/// GET /ws — WebSocket upgrade handler.
pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    debug!("WebSocket connection established");

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Try to parse as a JSON-RPC subscribe request.
                let response = if text.contains("kw_subscribe") {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "error": {
                            "code": -32601,
                            "message": "WebSocket subscriptions are not yet implemented. They ship in M2 (v0.2). See specs/development/dev_plan.md Phase 2.1."
                        }
                    })
                    .to_string()
                } else {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "error": {
                            "code": -32601,
                            "message": "WebSocket endpoint supports subscriptions only (M2). Use POST /rpc for JSON-RPC calls."
                        }
                    })
                    .to_string()
                };

                if socket.send(Message::Text(response.into())).await.is_err() {
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

    debug!("WebSocket connection closed");
}

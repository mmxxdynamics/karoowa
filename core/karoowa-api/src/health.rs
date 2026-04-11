//! Health and metrics endpoints.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use karoowa_storage::BlockStore;
use serde_json::Value;

use crate::state::AppState;

/// GET /health — liveness probe.
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let head_height = state.storage.head_height().unwrap_or(None).unwrap_or(0);
    let peer_count = state.network.peer_count();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "block_height": head_height,
            "peer_count": peer_count,
            "syncing": false,
        })),
    )
}

/// GET /metrics — Prometheus exposition format.
///
/// Uses the `metrics` facade with `metrics-exporter-prometheus`. In M1 we
/// expose a basic set of counters and gauges; the exporter is set up in
/// [`crate::server::start_server`].
pub async fn metrics_handler() -> String {
    // The PrometheusBuilder installs a global recorder. We use its render
    // method to produce the text output. If no recorder is installed (e.g.
    // in tests), return an empty string.
    let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
    recorder.handle().render()
}

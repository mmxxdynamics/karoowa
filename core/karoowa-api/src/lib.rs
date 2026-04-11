//! Karoowa API gateway.
//!
//! Single-port Axum service exposing JSON-RPC 2.0 (`/rpc`), REST (`/api/v1/*`),
//! WebSocket (`/ws`), health (`/health`), and Prometheus metrics (`/metrics`).
//!
//! All 14 `kw_*` JSON-RPC methods are implemented for M1. WebSocket
//! subscriptions ship in M2 (Phase 2.1).
//!
//! # Quick start
//!
//! ```no_run
//! use karoowa_api::server::{ServerConfig, start_server};
//! // Requires a RocksStorage and NetworkHandle — see server::start_server.
//! ```

pub mod error;
pub mod health;
pub mod rest;
pub mod rpc;
pub mod server;
pub mod state;
pub mod ws;

pub use error::ApiError;
pub use server::{build_router, start_server, ServerConfig};
pub use state::AppState;

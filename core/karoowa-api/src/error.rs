//! API error types.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Errors from the API layer.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("storage error: {0}")]
    Storage(#[from] karoowa_storage::StorageError),

    #[error("network error: {0}")]
    Network(#[from] karoowa_network::NetworkError),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("invalid params: {0}")]
    InvalidParams(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::InvalidParams(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

//! Service-level error type with axum `IntoResponse` integration.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Service-level error.
///
/// Converts into a JSON response of the form `{"error": "<message>"}` through
/// [`IntoResponse`]: [`AppError::NotFound`] gives 404, [`AppError::BadRequest`] gives 400,
/// and every other variant gives 500.
#[derive(Debug, Error)]
pub enum AppError {
    /// Configuration could not be loaded or was invalid. Responds 500.
    #[error("config error: {0}")]
    Config(String),

    /// The requested resource does not exist. Responds 404.
    #[error("not found")]
    NotFound,

    /// The request was malformed or invalid. Responds 400.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// An unexpected internal failure. Responds 500.
    #[error("internal error: {0}")]
    Internal(String),

    /// An I/O error, for example failing to bind the listen address in [`crate::run`]. Responds 500.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Config(_) | AppError::Internal(_) | AppError::Io(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

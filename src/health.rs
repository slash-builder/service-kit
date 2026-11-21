//! Health and readiness handlers.
//!
//! Mounted at `/healthz` and `/readyz` by [`crate::app::AppBuilder::build`],
//! inside the same default middleware stack as the caller's own routes.

use axum::http::StatusCode;

/// Liveness probe: always responds `200 OK`.
pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// Readiness probe: currently always responds `200 OK`; it does not check any dependency.
pub async fn readyz() -> StatusCode {
    StatusCode::OK
}

//! Health and readiness handlers.
//!
//! Mounted at `/healthz` and `/readyz` by [`crate::app::AppBuilder::build`],
//! inside the same default middleware stack as the caller's own routes.

use axum::http::StatusCode;

pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub async fn readyz() -> StatusCode {
    StatusCode::OK
}

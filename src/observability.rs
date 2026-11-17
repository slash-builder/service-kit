//! Observability — tracing initialization.
//!
//! [`init`] installs a global `tracing_subscriber` with JSON-formatted
//! output and an `EnvFilter` (defaults to `info`, overridable via
//! `RUST_LOG`).
//!
//! Metrics export (`metrics` + `metrics-exporter-prometheus`, already a
//! dependency of this crate) is **not** wired here yet — flagged as
//! deferred scope rather than half-built. Track it as a follow-up.

use tracing_subscriber::{fmt, EnvFilter};

use crate::AppError;

/// Initialize structured tracing for `service_name`. Idempotent: if a
/// global subscriber is already installed — a caller invoking `init` more
/// than once, or a test harness that beat this call to it — this is a
/// no-op rather than a panic or an error.
pub fn init(service_name: &str) -> Result<(), AppError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = fmt().with_env_filter(filter).json().finish();

    if tracing::subscriber::set_global_default(subscriber).is_ok() {
        tracing::info!(service = service_name, "service-kit: tracing initialized");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_ok_and_idempotent() {
        assert!(init("test-svc").is_ok());
        // Whether or not this call actually wins the race for the global
        // subscriber slot, it must not panic or error.
        assert!(init("test-svc").is_ok());
    }
}

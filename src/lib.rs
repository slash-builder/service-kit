//! # Service Kit
//!
//! axum scaffolding for backend services, part of the SlashBuilder Kit
//! ecosystem. Bundles middleware, structured logging, layered config, and
//! observability so every service starts from the same foundation.

pub mod app;
pub mod config;
pub mod error;
pub mod health;
pub mod observability;

pub use app::AppBuilder;
pub use error::AppError;

/// Crate version, sourced from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the assembled axum router until a shutdown signal arrives (Ctrl+C,
/// or on Unix, `SIGTERM`), then let in-flight requests finish before
/// returning.
///
/// Binds to the address in the `SERVICE_KIT_ADDR` env var, defaulting to
/// `0.0.0.0:8080`.
pub async fn run(app: axum::Router) -> Result<(), AppError> {
    let addr = std::env::var("SERVICE_KIT_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(AppError::Io)?;
    tracing::info!(%addr, "service-kit: listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(AppError::Io)
}

/// Resolves once a Ctrl+C (`SIGINT`) or, on Unix, `SIGTERM` is received.
///
/// This is the future [`run`] hands to axum's `with_graceful_shutdown`.
/// It is public so a service that must own its own listener and router
/// setup (a custom bind address, a WebSocket hub, a pre-built `Router`)
/// still gets the same signal handling instead of re-writing
/// it. The common mistake it exists to prevent is waiting on
/// `tokio::signal::ctrl_c()` alone: that handles only `SIGINT`, but
/// container orchestrators (ECS, Kubernetes, `docker stop`) send
/// `SIGTERM`, so graceful shutdown silently never runs and the process
/// is killed at the stop timeout.
///
/// ```no_run
/// # async fn demo(app: axum::Router) -> std::io::Result<()> {
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
/// axum::serve(listener, app)
///     .with_graceful_shutdown(service_kit::shutdown_signal())
///     .await
/// # }
/// ```
///
/// # Panics
///
/// Panics if the OS signal handler cannot be installed (as `tokio`'s own
/// signal API does). Handlers are installed on the first poll, not at
/// call time.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("service-kit: shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }

    #[tokio::test]
    async fn run_binds_serves_and_surfaces_bind_errors() {
        // Both cases go through one test function since they share the
        // process-global SERVICE_KIT_ADDR env var and would otherwise race
        // against each other under cargo's parallel test execution.

        // Happy path: bind to an ephemeral port and prove the server is
        // actually serving (still running when the timeout below fires),
        // without needing a real OS signal to stop it.
        // SAFETY: test-only env var; no other test touches it concurrently
        // because this whole scenario lives in one test function.
        unsafe {
            std::env::set_var("SERVICE_KIT_ADDR", "127.0.0.1:0");
        }
        let serving = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            run(axum::Router::new()),
        )
        .await;
        assert!(
            serving.is_err(),
            "expected run() to still be serving (and thus time out) absent a shutdown signal"
        );

        // Error path: an unparseable bind address should surface as
        // AppError::Io rather than panicking.
        unsafe {
            std::env::set_var("SERVICE_KIT_ADDR", "not-a-valid-address");
        }
        let result = run(axum::Router::new()).await;
        assert!(matches!(result, Err(AppError::Io(_))));

        unsafe {
            std::env::remove_var("SERVICE_KIT_ADDR");
        }
    }
}

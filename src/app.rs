//! axum `AppBuilder` — composes the default middleware stack.
//!
//! [`AppBuilder::build`] merges the caller's routes with the standard
//! [`crate::health`] probes, then wraps the result in a fixed middleware
//! stack. Layer order matters in `tower`/`axum`: layers added first to a
//! `ServiceBuilder` are outermost, so they see every request *before*, and
//! every response *after*, the layers added after them. This crate's
//! stack, outermost to innermost:
//!
//! 1. **request-id** (`SetRequestIdLayer`) — stamp `x-request-id` on the
//!    request before anything else runs, so every later layer can
//!    correlate by it. Never overrides an id a client/upstream proxy
//!    already supplied.
//! 2. **trace** (`tower_http::trace::TraceLayer`) — one `tracing` span per
//!    request/response (method, path, status, latency). Deliberately
//!    *not* `include_headers(true)`: that tower-http convenience logs
//!    every request header verbatim — including `authorization` — into
//!    structured JSON logs, which is a foot-gun for a default.
//! 3. **propagate request-id** (`PropagateRequestIdLayer`) — mirror the
//!    `x-request-id` header onto the response, after the handler runs but
//!    before the response re-crosses `trace`/the request-id setter, so
//!    the id is visible to trace output and to the caller.
//! 4. **CORS** (`tower_http::cors::CorsLayer::permissive()`) — a workable
//!    default for local dev / gateway-fronted deployments. Services that
//!    need a real origin allowlist should layer their own `CorsLayer` on
//!    top of the `Router` returned by `build()`.
//! 5. **compression** (`tower_http::compression::CompressionLayer`, gzip)
//!    — compress response bodies close to the wire.
//! 6. **timeout** (`tower_http::timeout::TimeoutLayer`) — bound handler
//!    execution time. Unlike `tower::timeout::Timeout`, tower-http's
//!    timeout layer doesn't change the wrapped service's error type — it
//!    resolves an elapsed timeout directly into a `408 Request Timeout`
//!    response, so it stays `Infallible` (as axum `Route`s already are)
//!    without needing an `axum::error_handling::HandleErrorLayer` pair.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{
    http::{HeaderName, HeaderValue, Request},
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

/// Default request timeout applied by [`AppBuilder::build`]. Override by
/// layering your own `TimeoutLayer` on the returned `Router` if a service
/// needs something different.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

fn request_id_header() -> HeaderName {
    HeaderName::from_static("x-request-id")
}

/// Generates request ids from a per-process monotonic counter. Cheap and
/// dependency-free (no `uuid` crate), and sufficient for correlating logs
/// within a single running process. `SetRequestIdLayer` never overrides an
/// id already present on the request.
#[derive(Clone, Default)]
struct CounterRequestId {
    next: Arc<AtomicU64>,
}

impl MakeRequestId for CounterRequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        HeaderValue::from_str(&format!("svc-{id:x}"))
            .ok()
            .map(RequestId::new)
    }
}

/// Builder for a service's axum [`Router`]: collects the service's own routes, then
/// [`AppBuilder::build`] adds the standard health routes and the default middleware stack.
pub struct AppBuilder {
    service_name: String,
    router: Router,
}

impl AppBuilder {
    /// Starts a builder for the service called `service_name`, with an empty router.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            router: Router::new(),
        }
    }

    /// Reserved builder stage for future service-scoped wiring (e.g.
    /// attaching loaded config as router state). Config loading itself
    /// lives in [`crate::config::load`], since its shape is
    /// service-specific — call it before `AppBuilder::new` and thread the
    /// result through your own routes' state.
    pub fn with_config(self) -> Result<Self, crate::AppError> {
        Ok(self)
    }

    /// Merges `routes` into the builder's router. May be called more than once; each call
    /// merges (see axum's `Router::merge` for how overlapping routes are handled).
    pub fn with_routes(mut self, routes: Router) -> Self {
        self.router = self.router.merge(routes);
        self
    }

    /// Merge in the standard health routes and wrap the result in the
    /// default middleware stack. See the module docs for the
    /// exact layer order and the reasoning behind it.
    pub fn build(self) -> Router {
        let health_routes = Router::new()
            .route("/healthz", get(crate::health::healthz))
            .route("/readyz", get(crate::health::readyz));

        let router = self.router.merge(health_routes);
        let request_id_header = request_id_header();

        let middleware = ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(
                request_id_header.clone(),
                CounterRequestId::default(),
            ))
            .layer(TraceLayer::new_for_http())
            .layer(PropagateRequestIdLayer::new(request_id_header))
            .layer(CorsLayer::permissive())
            .layer(CompressionLayer::new())
            .layer(TimeoutLayer::new(DEFAULT_TIMEOUT));

        tracing::debug!(
            service = %self.service_name,
            "service-kit: assembled app with default middleware stack"
        );

        router.layer(middleware)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::StatusCode};
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn health_routes_respond_ok() {
        let app = AppBuilder::new("test-svc").build();

        for path in ["/healthz", "/readyz"] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
        }
    }

    #[tokio::test]
    async fn request_id_is_stamped_and_propagated() {
        let app = AppBuilder::new("test-svc").build();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.headers().contains_key("x-request-id"),
            "expected x-request-id to be propagated onto the response"
        );
    }

    #[tokio::test]
    async fn cors_layer_is_applied() {
        let app = AppBuilder::new("test-svc").build();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .header("origin", "https://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response
                .headers()
                .contains_key("access-control-allow-origin"),
            "expected CorsLayer::permissive() to echo back an allow-origin header"
        );
    }

    #[tokio::test]
    async fn with_routes_merges_caller_routes_with_health() {
        let app = AppBuilder::new("test-svc")
            .with_routes(Router::new().route("/ping", get(|| async { "pong" })))
            .build();

        let response = app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

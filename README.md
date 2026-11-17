# Service Kit

> Opinionated axum + tower foundation for production Rust services.

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
![Status: pre-release](https://img.shields.io/badge/status-pre--release-orange.svg)

Service Kit is a small Rust library that wraps `axum`, `tower`,
structured tracing, layered configuration, and Prometheus-compatible
metrics into a curated starting point for new HTTP services. The goal:
no service built on it has to reinvent middleware ordering, request
logging, structured config, graceful shutdown, or the error model.

If you like axum but find yourself copy-pasting the same middleware
stack into every project, this is that stack as a crate. It's
opinionated — if you don't share the opinions, build your own; that's
fine.

## Status

Pre-1.0. `AppBuilder`, `AppError`, the default middleware stack, layered
config loading, `observability::init`, and `run` (graceful shutdown) are
implemented. Prometheus metrics export (`metrics` +
`metrics-exporter-prometheus`, already a dependency) is not yet wired to
an endpoint. Refraction's Go → Rust/axum pivot is the first real
consumer and will drive further requirements.

Treat current versions as a design preview. Pin to an exact version if
you choose to depend on it.

## Quick start

```toml
[dependencies]
slashbuilder-service-kit = "0.1"
tokio                    = { version = "1", features = ["macros", "rt-multi-thread"] }
axum                     = "0.7"
```

The package is `slashbuilder-service-kit`; the library is imported as
`service_kit` (the unprefixed `service-kit` name on crates.io belongs to
an unrelated crate).

```rust
use axum::{Router, routing::get};
use service_kit::{AppBuilder, AppError};

async fn hello() -> &'static str { "hello, world" }

#[tokio::main]
async fn main() -> Result<(), AppError> {
    service_kit::observability::init("my-service")?;

    let routes = Router::new().route("/", get(hello));

    let app = AppBuilder::new("my-service")
        .with_config()?
        .with_routes(routes)
        .build();

    service_kit::run(app).await
}
```

By default the assembled app provides request-id propagation, tracing
spans per request, CORS, gzip compression, request timeout, and
`/healthz` + `/readyz` endpoints. Override by composing your own
middleware on top.

## The stack

- **`axum` 0.7** — routing and handler ergonomics.
- **`tower` + `tower-http`** — middleware (trace, cors, compression,
  request-id, timeout).
- **`hyper` 1** — HTTP transport.
- **`tokio`** — async runtime.
- **`tracing` + `tracing-subscriber`** — structured logging
  (JSON-by-default in production).
- **`figment`** — layered config loading
  (defaults → file → environment).
- **`metrics` + `metrics-exporter-prometheus`** — Prometheus-compatible
  metrics on a dedicated endpoint.

## Scope

**In scope.** Middleware composition with sensible defaults. Layered
configuration. Structured logging + metrics initialization. Graceful
shutdown. An `AppError` type that satisfies `IntoResponse` so handler
code can `?` cleanly.

**Out of scope.** Your routes, your business logic, your data model.
Authentication — that's the
SlashBuilder Identity Kit's concern, and this crate does not depend on it.
Database setup — service
crates wire their own ORM or driver.

## Building from source

```bash
git clone https://github.com/slash-builder/service-kit.git
cd service-kit
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
```

Requires Rust 1.74 or newer (the minimum supported Rust version is checked
in CI against the committed `Cargo.lock`).

## Contributing

Contributions welcome under Apache-2.0. Sign your commits with
`git commit -s` (Developer Certificate of Origin). See
[CONTRIBUTING.md](CONTRIBUTING.md).

The default middleware stack and its ordering are the load-bearing
opinions — proposals to change them should start as an issue with a
concrete use case attached.

## License

Apache-2.0 — see [LICENSE](LICENSE).

## About

Service Kit is part of the **SlashBuilder Kit ecosystem** — a set of
reusable Rust and Dart libraries built around the BeOS-style "Kit"
pattern. Each Kit owns a self-contained domain; products are composed
from Kits. First consumer is
Refraction, the bitchain
server-side reference implementation.

Maintained by [SlashBuilder](https://slashbuilder.com) / Douglas Lockamy.

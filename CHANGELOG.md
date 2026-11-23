# Changelog

All notable changes to **service-kit** will be documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/slash-builder/service-kit/compare/v0.1.0...v0.1.1) - 2026-11-23

### Fixed

- *(deps)* update url to 2.5.9 (RUSTSEC-2026-0592) ([#13](https://github.com/slash-builder/service-kit/pull/13))

### Other

- *(deps)* Bump tokio from 1.53.3 to 1.54.1 ([#7](https://github.com/slash-builder/service-kit/pull/7))
- raise the minimum supported Rust version to 1.85 ([#12](https://github.com/slash-builder/service-kit/pull/12))
- document the public API; forbid unsafe code outside tests ([#10](https://github.com/slash-builder/service-kit/pull/10))
- *(deps)* Bump reqwest from 0.12.28 to 0.13.5 ([#3](https://github.com/slash-builder/service-kit/pull/3))

### Added

- First public release as the `slashbuilder-service-kit` package. The library
  name is unchanged: import it as `service_kit`. (The unprefixed `service-kit`
  name on crates.io belongs to an unrelated crate.)
- `AppBuilder` (`new`, `with_config`, `with_routes`, `build`), which merges the
  caller's routes with `/healthz` and `/readyz` and wraps them in the default
  middleware stack: request-id, tracing, request-id propagation, CORS, gzip
  compression and a request timeout.
- `AppError` with axum `IntoResponse` integration (JSON error body and status
  mapping).
- Layered configuration loading (defaults, file, environment) in `config`.
- `observability::init`: structured JSON tracing with an `EnvFilter`.
- `run(Router)`: serves until Ctrl-C or, on Unix, `SIGTERM`, then lets
  in-flight requests finish.
- `service_kit::shutdown_signal()` is public, so services that own their own
  listener and router can reuse the Ctrl-C plus `SIGTERM` future instead of
  waiting on `tokio::signal::ctrl_c()` alone, which never fires under
  container orchestrators that send `SIGTERM`. Covered by a `SIGTERM`
  integration test (`tests/shutdown_signal.rs`, Unix only).

### Notes

- Prometheus metrics export (`metrics` and `metrics-exporter-prometheus` are
  dependencies) is not yet wired to an endpoint.
- Minimum supported Rust version: 1.74.

[Unreleased]: https://github.com/slash-builder/service-kit/commits/main

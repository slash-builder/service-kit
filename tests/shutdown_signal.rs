//! `SIGTERM` handling, end to end.
//!
//! These live in their own integration-test binary on purpose: a signal is
//! process-global, so sending `SIGTERM` from a test inside the library's unit
//! tests would also wake the unrelated `run()` test that is serving at the
//! same moment. Here one `#[test]` runs the phases strictly in sequence.
//!
//! Unix-only: the subject is `SIGTERM`, which Windows does not have.

#![cfg(unix)]

use std::process::Command;
use std::time::Duration;

/// Send `SIGTERM` to this very process via `kill(1)` (no `libc` dependency).
fn sigterm_self() {
    let status = Command::new("kill")
        .args(["-TERM", &std::process::id().to_string()])
        .status()
        .expect("spawn kill(1)");
    assert!(status.success(), "kill -TERM failed: {status}");
}

#[tokio::test]
async fn sigterm_resolves_shutdown_signal_and_ends_run_cleanly() {
    // Phase 1: the public future resolves on SIGTERM. If no handler were
    // installed, the default disposition would kill the whole test process,
    // so a passing run also proves the handler exists.
    let waiter = tokio::spawn(service_kit::shutdown_signal());
    // Handlers are installed on first poll; give the spawned task a turn.
    tokio::time::sleep(Duration::from_millis(200)).await;
    sigterm_self();
    tokio::time::timeout(Duration::from_secs(5), waiter)
        .await
        .expect("shutdown_signal() must resolve within 5s of SIGTERM")
        .expect("shutdown task must not panic");

    // Phase 2: `run()` serves, then returns Ok(()) after SIGTERM (a graceful
    // stop, not an error and not a hang).
    // SAFETY: test-only env var; this binary has a single test function.
    unsafe {
        std::env::set_var("SERVICE_KIT_ADDR", "127.0.0.1:0");
    }
    let server = tokio::spawn(service_kit::run(axum::Router::new()));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !server.is_finished(),
        "run() must keep serving until signalled"
    );
    sigterm_self();
    let result = tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("run() must return within 5s of SIGTERM")
        .expect("server task must not panic");
    assert!(
        result.is_ok(),
        "graceful shutdown must be Ok, got {result:?}"
    );
}

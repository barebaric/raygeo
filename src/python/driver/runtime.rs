//! The shared tokio runtime backing all sessions.
//!
//! A single process-wide runtime keeps task spawning cheap and
//! avoids interpreter-shutdown deadlocks: per-session runtimes would
//! have to be joined during `Drop`, which cannot block on tasks that
//! need the GIL while Python is finalizing.
//!
//! The runtime must not outlive the interpreter: a worker thread
//! that still holds a Python thread state at `PyInterpreterState_Delete`
//! time aborts the process ("remaining threads").  [`shutdown`]
//! therefore tears the runtime down from an `atexit` hook, which
//! CPython runs before interpreter finalization.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

static RUNTIME: Mutex<Option<tokio::runtime::Runtime>> = Mutex::new(None);

static HANDLE: LazyLock<tokio::runtime::Handle> = LazyLock::new(|| {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");
    spawn_gil_watchdog(&runtime);
    let handle = runtime.handle().clone();
    *RUNTIME.lock().unwrap() = Some(runtime);
    handle
});

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Heartbeat that periodically attaches to the GIL and prints a
/// line.  If the interpreter ever deadlocks on the GIL, the
/// heartbeats stop, which the CI logs will show.
///
/// Opt-in via ``RAYDRIVER_GIL_WATCHDOG=1``.  Disabled by default:
/// this is a temporary freeze diagnostic, and the periodic
/// ``PyGILState_Ensure`` from a tokio worker thread can race with
/// session teardown on Windows and crash the interpreter with an
/// access violation.  The freeze it was added to diagnose is fixed
/// by the atexit runtime shutdown below, so the heartbeat is no
/// longer needed in normal runs.
fn spawn_gil_watchdog(runtime: &tokio::runtime::Runtime) {
    if std::env::var("RAYDRIVER_GIL_WATCHDOG").ok().as_deref() != Some("1") {
        return;
    }
    runtime.spawn(async {
        let start = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            if SHUTDOWN.load(Ordering::Relaxed) {
                break;
            }
            let secs = start.elapsed().as_secs();
            let attached_at = std::time::Instant::now();
            pyo3::Python::attach(|py| {
                let _ = py;
                eprintln!(
                    "[gil-watchdog] alive after {secs}s (attach took \
                     {:?})",
                    attached_at.elapsed()
                );
            });
        }
    });
}

pub(crate) fn handle() -> &'static tokio::runtime::Handle {
    &HANDLE
}

/// Cancel all tasks and join the worker threads.  Registered as an
/// `atexit` hook by [`super::register`]; safe to call more than
/// once, and a no-op if the runtime was never used.
pub(crate) fn shutdown() {
    SHUTDOWN.store(true, Ordering::Relaxed);
    if let Ok(mut guard) = RUNTIME.lock() {
        if let Some(runtime) = guard.take() {
            runtime.shutdown_timeout(Duration::from_secs(2));
        }
    }
}

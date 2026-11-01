/// Debug-logging macro: prints to stderr in dev builds, no-op in release.
///
/// Dev builds emit only when `RAYGEO_DEBUG=1`: the engine's step tracing is
/// for debugging raygeo itself, and a downstream `cargo test` compiles with
/// debug assertions on, where unconditional printing would flood its output.
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! dbg_log {
    ($($arg:tt)*) => {
        if $crate::log::debug_enabled() {
            eprintln!($($arg)*);
        }
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! dbg_log {
    ($($arg:tt)*) => {
        // Drop the format arguments so values used only for logging
        // are still considered "used" by the compiler.
        #[allow(unused_must_use, unused_variables, unused_assignments)]
        {
            if false {
                let _ = format_args!($($arg)*);
            }
        }
    };
}

/// Whether dev-build tracing is switched on (`RAYGEO_DEBUG=1`).
#[cfg(debug_assertions)]
pub fn debug_enabled() -> bool {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED
        .get_or_init(|| std::env::var("RAYGEO_DEBUG").is_ok_and(|v| v == "1"))
}

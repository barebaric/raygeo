/// Debug-logging macro: prints to stderr in dev builds, no-op in release.
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! dbg_log {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
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

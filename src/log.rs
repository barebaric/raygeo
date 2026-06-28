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
    ($($arg:tt)*) => {};
}

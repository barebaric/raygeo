//! Shared utility functions used across crate layers.

/// Sort comparison helper for `f64`: unwraps `partial_cmp` with `Equal`
/// fallback, suitable for `sort_by` closures.
pub fn sort_f64(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

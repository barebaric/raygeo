//! Assembler-owned caching types.
//!
//! Types shared between the [`Assembler`](crate::ops::assembly::Assembler)
//! trait and the consumer that holds the cache. Lives in `ops` (not the
//! consumer's crate) so the `Assembler` trait can reference them
//! without an upward import.
//!
//! The [`Cacheable`] trait is the uniform caching contract for every
//! caching-aware component: assemblers, transformers, and encoders
//! each implement `Cacheable<TheirOutput>` with independent
//! [`cache_key`](Cacheable::cache_key) /
//! [`store_cache`](Cacheable::store_cache) /
//! [`restore_cache`](Cacheable::restore_cache) logic.

use crate::ops::container::Ops;
use crate::ops::part::FaceState;
use crate::types::Polygon;

/// Cache key: a caller-provided `tag` plus a component-computed hash
/// of its read set.
///
/// Each component computes `payload_hash` from exactly the fields it
/// reads (spec fields + optional face state fields), so a change to a
/// non-read field does not invalidate the entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Caller-provided identifier, used for prefix-based pruning
    /// (e.g. `"workpiece-42"`) and for matching.
    pub tag: String,
    /// Component-computed hash of its read-set fields. The consumer
    /// does not interpret this value; it only compares for equality.
    pub payload_hash: u64,
}

impl CacheKey {
    /// Construct a new cache key.
    pub fn new(tag: impl Into<String>, payload_hash: u64) -> Self {
        CacheKey {
            tag: tag.into(),
            payload_hash,
        }
    }
}

/// The output of an assembler, packaged for caching.
///
/// Carries the assembled `Ops`, metadata, and optional post-assembly
/// cleared fragments for face-state restoration on cache hit.
#[derive(Debug, Clone)]
pub struct AssemblyOutput {
    /// The assembled `Ops` (with transformers already applied).
    pub ops: Ops,
    /// Whether the `Ops` may be uniformly scaled during aggregation.
    pub is_scalable: bool,
    /// Source `(width_mm, height_mm)` of the part that produced `ops`.
    pub source_dimensions: Option<(f64, f64)>,
    /// Post-assembly cleared fragments to restore into
    /// `FaceState.cleared`. `None` for assemblers that don't touch
    /// `cleared`.
    pub cleared_fragments: Option<Vec<Polygon>>,
}

impl AssemblyOutput {
    /// Produce the core output triple `(ops, is_scalable,
    /// source_dimensions)` for the consumer to build its own
    /// output type without an upward dependency on `AssemblyOutput`.
    pub fn into_parts(self) -> (Ops, bool, Option<(f64, f64)>) {
        (self.ops, self.is_scalable, self.source_dimensions)
    }
}

/// Uniform caching contract for components that produce cacheable
/// output.
///
/// Every component that can produce cacheable output (assemblers,
/// transformers, encoders) implements `Cacheable<TCached>` where
/// `TCached` is the component's output type:
///
/// | Component | `TCached` | Description |
/// |---|---|---|
/// | [`Assembler`](crate::ops::assembly::Assembler) | [`AssemblyOutput`] | ops + metadata + cleared fragments |
/// | [`Transformer`](crate::ops::transform::Transformer) | [`Ops`] | transformed ops (same type in and out) |
/// | [`Encoder`](crate::ops::convert::Encoder) | [`EncodeOutput`](crate::ops::convert::EncodeOutput) | encoded bytes + metadata |
///
/// Default implementations opt out (return `None`) so existing specs
/// automatically opt out until an explicit `impl Cacheable<T>` is
/// added.
pub trait Cacheable<TCached: Send + 'static>: Send + Sync {
    /// Compute a cache key for this component.
    ///
    /// `face` is `Some` for assemblers (which read face state) and
    /// `None` for transformers and encoders (which only read their
    /// own spec fields). Returns `None` to opt out of caching.
    fn cache_key(
        &self,
        _face: Option<&FaceState>,
        _tag: &str,
    ) -> Option<CacheKey> {
        None
    }

    /// Reconstruct a cached value from the stored entry.
    ///
    /// The default returns `None` (opt-out). Opt-in components
    /// typically return `Some(cached.clone())`.
    fn restore_cache(&self, _cached: &TCached) -> Option<TCached> {
        None
    }

    /// Build a cacheable value from the component's output.
    ///
    /// The default returns `None` (opt-out). Opt-in components
    /// typically return `Some(output.clone())`.
    fn store_cache(&self, _output: &TCached) -> Option<TCached> {
        None
    }
}

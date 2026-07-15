use std::collections::HashSet;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

use prof_macros::prof;

use super::crescent;
use super::stock_region::StockRegion;
use crate::geo::algo::engagement::Engagement;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::algo::simplify::simplify_polyline;
use crate::geo::algo::spatial_grid2d::SpatialGrid;
use crate::geo::shape::polygon::{
    get_polygon_bounds, get_polygon_signed_area, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_union,
    get_polyline_swept_polygon, get_segment_swept_polygon, offset_polygon,
    JoinStyle,
};
use crate::types::{Point, Polygon, Rect};

/// Small LRU cache for [`ClearedArea::cut_area_split`].
///
/// During a step evaluation the stepper probes 6-7 different angles
/// (varying `c2`), and adjacent steps often probe overlapping angle
/// ranges.  A single-entry cache only catches repeated hits on the
/// converged angle; a small multi-entry cache also catches adjacent
/// steps that revisit nearby angles.
///
/// Uses a fixed-size array with a monotonically increasing generation
/// counter per entry to avoid `HashMap` overhead.
struct SweepCache {
    entries: [SweepEntry; 8],
    /// The generation counter is bumped on every insertion.  The entry
    /// with the lowest generation is evicted on a miss.
    gen: u64,
}

struct SweepEntry {
    c1: Point,
    c2: Point,
    radius: f64,
    total: f64,
    left: f64,
    generation: u64,
    valid: bool,
}

impl SweepCache {
    const EMPTY: Self = SweepCache {
        entries: [
            SweepEntry::EMPTY,
            SweepEntry::EMPTY,
            SweepEntry::EMPTY,
            SweepEntry::EMPTY,
            SweepEntry::EMPTY,
            SweepEntry::EMPTY,
            SweepEntry::EMPTY,
            SweepEntry::EMPTY,
        ],
        gen: 0,
    };

    /// Look up a cached `(c1, c2, radius)` triple.
    #[inline]
    fn get(&mut self, c1: Point, c2: Point, radius: f64) -> Option<(f64, f64)> {
        for e in &mut self.entries {
            if e.valid
                && e.c1 == c1
                && e.c2 == c2
                && (e.radius - radius).abs() < 1e-12
            {
                e.generation = self.gen;
                self.gen += 1;
                return Some((e.total, e.left));
            }
        }
        None
    }

    /// Insert a result, evicting the LRU entry if full.
    #[inline]
    fn insert(
        &mut self,
        c1: Point,
        c2: Point,
        radius: f64,
        total: f64,
        left: f64,
    ) {
        // Try to find an empty slot first.
        let slot = self.entries.iter_mut().find(|e| !e.valid);
        let slot = match slot {
            Some(s) => s,
            None => {
                // Evict the entry with the lowest generation.
                self.entries
                    .iter_mut()
                    .min_by_key(|e| e.generation)
                    .unwrap()
            }
        };
        slot.c1 = c1;
        slot.c2 = c2;
        slot.radius = radius;
        slot.total = total;
        slot.left = left;
        slot.generation = self.gen;
        self.gen += 1;
        slot.valid = true;
    }

    /// Invalidate all entries (called on fragment mutation).
    #[inline]
    fn clear(&mut self) {
        for e in &mut self.entries {
            e.valid = false;
        }
    }
}

impl SweepEntry {
    const EMPTY: Self = SweepEntry {
        c1: Point { x: 0.0, y: 0.0 },
        c2: Point { x: 0.0, y: 0.0 },
        radius: 0.0,
        total: 0.0,
        left: 0.0,
        generation: 0,
        valid: false,
    };
}

/// Cached nearby fragment list for [`ClearedArea::cut_area_split`].
///
/// Within a single step, `c1` is fixed and fragments don't change —
/// only the angle (and thus `c2`) varies.  The `query_window` call
/// returns the same fragment set for all angles because the step
/// length is small relative to the query window.  Caching this list
/// avoids the redundant spatial query on each angle iteration.
struct NearbyCache {
    c1: Point,
    radius: f64,
    nearby: Vec<Polygon>,
}

pub struct ClearedArea {
    pub(crate) fragments: Vec<Polygon>,
    pub(crate) grid: SpatialGrid,
    pub(crate) cell_size: f64,
    // ── Batched step expansion ──
    /// Path points accumulated while a batch is open.
    /// At commit time these are turned into a single swept polygon
    /// via [`get_polyline_swept_polygon`].
    pub(crate) batch_path: Vec<Point>,
    /// Tool radius for the current batch (set by the first expand call).
    pub(crate) batch_radius: f64,
    /// True when `begin_batch()` has been called.
    pub(crate) batch_active: bool,
    // ── Sweep cache ──
    /// Multi-entry LRU result cache for [`Self::cut_area_split`].
    /// Cleared whenever fragments are mutated (see
    /// [`Self::clear_sweep_cache`]).
    sweep_cache: Mutex<SweepCache>,
    /// Cached nearby fragment list for [`Self::cut_area_split`].
    /// Keyed by `(c1, radius)` — avoids redundant `query_window` calls
    /// when only the angle changes within a step.
    nearby_cache: Mutex<Option<NearbyCache>>,
    /// Cached `get_polygons_union(&self.fragments)`.  Updated
    /// incrementally instead of recomputed from scratch on every
    /// mutation — avoids redundant full union in both [`Self::frontier`]
    /// and [`Self::actionable_remaining`].
    fragments_union_cache: Mutex<Option<Vec<Polygon>>>,
    /// Cached result of [`Self::remaining_area`].  Invalidated when
    /// fragments change.
    remaining_area_cache: Mutex<Option<f64>>,
    /// Cached result of [`Self::actionable_remaining`] keyed by
    /// `(inset_distance, fragment_version)`.  Avoids recomputing the
    /// inset-region difference when fragments haven't changed.
    actionable_cache: Mutex<Option<(f64, usize, f64)>>,
    /// Version counter bumped on every fragment mutation.  Used by
    /// [`Self::actionable_remaining`] to detect staleness without
    /// locking the union cache.
    frag_version: AtomicUsize,
    // ── Envelope cache ──
    /// The tool-centre envelope is constant for a given pocket (it
    /// depends only on the stock boundary and tool radius, not on the
    /// fragments).  Cached here to avoid recomputing
    /// [`compute_inset_region`] on every `bites()` and
    /// `actionable_remaining()` during the wavefront loop.
    envelope_cache: Mutex<Option<Vec<Polygon>>>,
}

impl ClearedArea {
    // ── Construction ───────────────────────────────────────────────

    /// Create an empty cleared area.
    pub fn new() -> Self {
        let cell_size = 10.0;
        ClearedArea {
            fragments: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            batch_path: Vec::new(),
            batch_radius: 0.0,
            batch_active: false,
            sweep_cache: Mutex::new(SweepCache::EMPTY),
            nearby_cache: Mutex::new(None),
            fragments_union_cache: Mutex::new(None),
            remaining_area_cache: Mutex::new(None),
            actionable_cache: Mutex::new(None),
            frag_version: AtomicUsize::new(0),
            envelope_cache: Mutex::new(None),
        }
    }

    /// Create a cleared area pre-seeded with `initial` polygons.
    /// `initial` is **not** clipped.
    #[prof]
    pub fn with_fragments(initial: &[Polygon]) -> Self {
        let mut ca = Self::new();
        for poly in initial {
            if poly.len() >= 3 {
                let p = poly.clone();
                let idx = ca.fragments.len();
                ca.grid.insert(idx, get_polygon_bounds(&p));
                ca.fragments.push(p);
            }
        }
        ca
    }

    // ── Stock helpers ──────────────────────────────────────────────

    /// The stock shape: boundary ∖ islands.
    #[prof]
    fn stock(&self, region: &StockRegion) -> Vec<Polygon> {
        if region.boundary.len() < 3 {
            return vec![];
        }
        if region.islands.is_empty() {
            vec![region.boundary.clone()]
        } else {
            get_polygons_group_difference(
                std::slice::from_ref(&region.boundary),
                &region.islands,
            )
        }
    }

    /// The tool-centre envelope (inset of boundary by `tool_radius`,
    /// minus islands).
    ///
    /// Uses the cached envelope if one has been set via
    /// [`set_envelope_cache`](Self::set_envelope_cache), which avoids
    /// recomputing [`compute_inset_region`] on every call during the
    /// wavefront loop.
    #[prof]
    pub fn envelope(
        &self,
        region: &StockRegion,
        tool_radius: f64,
    ) -> Vec<Polygon> {
        {
            let cache = self.envelope_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                return cached.clone();
            }
        }
        compute_inset_region(&region.boundary, tool_radius, &region.islands).0
    }

    /// Pre-compute and cache the envelope so that
    /// [`Self::envelope`] returns it without recomputation.
    pub fn set_envelope_cache(&self, envelope: Vec<Polygon>) {
        *self.envelope_cache.lock().unwrap() = Some(envelope);
    }

    /// Clear the envelope cache (e.g. when the tool radius or stock
    /// region changes).
    pub fn clear_envelope_cache(&self) {
        *self.envelope_cache.lock().unwrap() = None;
    }

    /// Find a safe plunge point in the cleared area near `near`.
    ///
    /// Wraps [`find_plunge_point`](crate::ops::feature::near::find_plunge_point)
    /// using this area's fragments, boundary, and islands.
    pub fn find_plunge_point(
        &self,
        region: &StockRegion,
        near: Point,
        tool_radius: f64,
        search_radius: f64,
    ) -> Option<Point> {
        crate::ops::feature::near::find_plunge_point(
            near,
            &self.fragments,
            &region.boundary,
            &region.islands,
            tool_radius,
            search_radius,
        )
    }

    // ── Mutation ───────────────────────────────────────────────────

    /// Clear sweep, remaining-area, and actionable caches (called
    /// whenever fragments are modified).  The `fragments_union_cache` is
    /// NOT cleared here — it is updated incrementally via
    /// [`Self::update_fragments_union`] to avoid re-unioning all
    /// fragments from scratch.
    #[inline]
    fn clear_sweep_cache(&self) {
        self.sweep_cache.lock().unwrap().clear();
        *self.nearby_cache.lock().unwrap() = None;
        *self.remaining_area_cache.lock().unwrap() = None;
        *self.actionable_cache.lock().unwrap() = None;
        self.frag_version.store(
            self.frag_version.load(Ordering::Relaxed) + 1,
            Ordering::Relaxed,
        );
    }

    /// Incrementally update the cached union of all fragments.
    ///
    /// `new_polys` are polygons that have just been added (and were NOT
    /// already in `self.fragments`).  The new union is
    /// `union(old_union, new_polys)`.  While `new_polys` don't overlap
    /// the existing cleared area, individual polygons within `new_polys`
    /// may overlap each other, so we must actually perform the Clipper
    /// union to keep the cached union clean.  This is still much cheaper
    /// than re-unioning all fragments from scratch.
    fn update_fragments_union(&self, new_polys: &[Polygon]) {
        if new_polys.is_empty() {
            return;
        }
        let filtered: Vec<&Polygon> =
            new_polys.iter().filter(|p| p.len() >= 3).collect();
        if filtered.is_empty() {
            return;
        }
        let mut cache = self.fragments_union_cache.lock().unwrap();
        let current = cache.take();
        let updated = match current {
            Some(mut u) => {
                u.extend(filtered.into_iter().cloned());
                get_polygons_union(&u)
            }
            None => {
                let mut all = self.fragments.clone();
                all.extend(new_polys.iter().filter(|p| p.len() >= 3).cloned());
                get_polygons_union(&all)
            }
        };
        *cache = Some(updated);
    }

    /// Convenience: (re)build the union cache from fragments directly.
    /// Used when fragments are replaced wholesale (e.g. `replace_fragments`).
    fn rebuild_union_cache(&self) {
        *self.fragments_union_cache.lock().unwrap() =
            Some(get_polygons_union(&self.fragments));
    }

    /// Replace all stored fragments with a new set (e.g., the new frontier
    /// after a wavefront advance).  This is O(m) in the new fragment count
    /// and avoids the O(n·m) accumulation of
    /// [`cut`](Self::cut).
    #[prof]
    pub fn replace_fragments(&mut self, fragments: Vec<Polygon>) {
        self.fragments = fragments;
        self.clear_sweep_cache();
        self.rebuild_union_cache();
        self.rebuild_grid();
    }

    /// Return the signed perpendicular distance from `(x, y)` to the
    /// nearest cleared‑area boundary.
    ///
    /// * **Positive** — the point is **outside** the cleared area
    ///   (in uncut material).
    /// * **Negative** — the point is **inside** the cleared area (in
    ///   the void).
    /// * `0.0` — the point lies exactly on the boundary.
    ///
    /// Uses `get_polygons_closest_point` for the exact unsigned distance,
    /// then checks point-in-polygon to determine the sign.
    pub fn signed_boundary_distance(&self, x: f64, y: f64) -> f64 {
        crate::geo::shape::polygon::get_signed_boundary_distance(
            Point::new(x, y),
            &self.fragments,
        )
    }

    /// Merge swept polygons into `self.fragments` via
    /// `get_polygons_union` and rebuild the spatial grid.
    #[prof]
    fn union_swept(&mut self, swept: Vec<Polygon>) {
        if swept.is_empty() || swept.iter().all(|p| p.len() < 3) {
            return;
        }
        // Update union cache before consuming swept.
        self.update_fragments_union(&swept);
        let mut all_polys = self.fragments.clone();
        all_polys.extend(swept);
        self.fragments = get_polygons_union(&all_polys);
        self.clear_sweep_cache();
        self.rebuild_grid();
    }

    #[prof]
    pub fn expand(&mut self, path: &[Point], radius: f64) {
        if path.len() < 2 || radius < 1e-12 {
            return;
        }
        let mut swept_polys: Vec<Polygon> = Vec::new();
        for window in path.windows(2) {
            swept_polys.extend(get_segment_swept_polygon(
                window[0], window[1], radius,
            ));
        }
        self.union_swept(swept_polys);
    }

    #[prof]
    pub fn expand_step(&mut self, prev: Point, next: Point, radius: f64) {
        let swept = get_segment_swept_polygon(prev, next, radius);
        self.union_swept(swept);
    }

    #[prof]
    pub fn query_window(&self, bbox: Rect) -> Vec<&Polygon> {
        let indices = self.grid.query(bbox);
        indices
            .iter()
            .filter_map(|&idx| self.fragments.get(idx))
            .collect()
    }

    /// Return the uncut stock: stock ∖ fragments.
    ///
    /// Polygons below 0.01 mm² are dropped as Clipper2 numerical artifacts.
    #[prof]
    pub fn remaining(&self, region: &StockRegion) -> Vec<Polygon> {
        let stock = self.stock(region);
        if self.fragments.is_empty() {
            return stock;
        }
        if stock.is_empty() {
            return vec![];
        }
        let result = get_polygons_group_difference(&stock, &self.fragments);
        result
            .into_iter()
            .filter(|p| {
                p.len() >= 3 && get_polygon_signed_area(p).abs() >= 0.01
            })
            .collect()
    }

    /// Area of uncut material remaining in the pocket.
    ///
    /// Computed as stock area minus the signed area of fragments
    /// intersected with the stock.  This avoids depending on the
    /// flat-path output of the difference operation (which can
    /// produce orphan CCW outers around islands).
    #[prof]
    pub fn remaining_area(&self, region: &StockRegion) -> f64 {
        let mut cache = self.remaining_area_cache.lock().unwrap();
        if let Some(cached) = *cache {
            return cached;
        }
        let stock = self.stock(region);
        let result = if stock.is_empty() {
            0.0
        } else if self.fragments.is_empty() {
            stock.iter().map(|p| get_polygon_signed_area(p)).sum()
        } else {
            let stock_area: f64 =
                stock.iter().map(|p| get_polygon_signed_area(p)).sum();
            let in_stock =
                get_polygons_group_intersection(&stock, &self.fragments);
            let cleared: f64 =
                in_stock.iter().map(|p| get_polygon_signed_area(p)).sum();
            (stock_area - cleared).max(0.0)
        };
        *cache = Some(result);
        result
    }

    #[prof]
    pub fn total_area(&self) -> f64 {
        let total: f64 = self
            .fragments
            .iter()
            .map(|p| get_polygon_signed_area(p))
            .sum();
        total.max(0.0)
    }
    /// Area of uncleared material **inside the actionable zone**.
    ///
    /// The actionable zone is the pocket boundary inset by
    /// `inset_distance` (with islands buffered by the same amount).
    /// Any material outside this zone — wall-band slivers thinner
    /// than `inset_distance` — is excluded from the residual because
    /// the stepper cannot productively engage with it.
    ///
    /// For convergence gating, `inset_distance` is typically
    /// `step_length`: slivers thinner than the per-step advance get
    /// skipped by the stepper, so they should not block convergence.
    ///
    /// Computed lazily on each call as
    /// `area(inset_region − fragments_union)`.
    #[prof]
    pub fn actionable_remaining(
        &self,
        region: &StockRegion,
        inset_distance: f64,
    ) -> f64 {
        let version = self.frag_version.load(Ordering::Relaxed);
        {
            let cache = self.actionable_cache.lock().unwrap();
            if let Some((d, v, result)) = *cache {
                if (d - inset_distance).abs() < 1e-9 && v == version {
                    return result;
                }
            }
        }
        let inset_region = {
            // Reuse the cached envelope when the inset distance matches
            // the tool radius (common in wavefront loops).
            let env = self.envelope_cache.lock().unwrap();
            if let Some(ref cached) = *env {
                cached.clone()
            } else {
                drop(env);
                compute_inset_region(
                    &region.boundary,
                    inset_distance,
                    &region.islands,
                )
                .0
            }
        };
        let result = if inset_region.is_empty() {
            0.0
        } else if self.fragments.is_empty() {
            inset_region
                .iter()
                .map(|p| get_polygon_signed_area(p))
                .sum::<f64>()
                .max(0.0)
        } else {
            let unclipped = {
                let mut cache = self.fragments_union_cache.lock().unwrap();
                let unioned = cache
                    .get_or_insert_with(|| get_polygons_union(&self.fragments));
                get_polygons_group_difference(&inset_region, unioned)
            };
            let total: f64 = unclipped
                .into_iter()
                .filter(|p| {
                    p.len() >= 3 && get_polygon_signed_area(p).abs() >= 0.01
                })
                .map(|p| get_polygon_signed_area(&p))
                .sum();
            total.max(0.0)
        };
        *self.actionable_cache.lock().unwrap() =
            Some((inset_distance, version, result));
        result
    }

    #[prof]
    pub fn fragments(&self) -> &[Polygon] {
        &self.fragments
    }

    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Directly insert known cleared polygons. This avoids the overhead
    /// of sweeping thousands of individual line segments.
    #[prof]
    pub fn cut(&mut self, polygons: &[Polygon]) {
        if polygons.is_empty() {
            return;
        }
        let mut all_polys = self.fragments.clone();
        all_polys.extend(polygons.iter().cloned());
        self.fragments = get_polygons_union(&all_polys);
        self.update_fragments_union(polygons);
        self.clear_sweep_cache();
        self.rebuild_grid();
    }

    /// Add polygons and return only the newly-added portion (input minus
    /// already-cleared area).  When bounding boxes overlap a local merge
    /// unions only the overlapping fragments with the new geometry.
    /// A full compaction (union of all fragments) is triggered only when
    /// the total vertex count exceeds the threshold, preventing OOM without
    /// paying the full union cost on every call.
    #[prof]
    pub fn cut_fast(&mut self, polys: &[Polygon]) -> Vec<Polygon> {
        if polys.is_empty() {
            return vec![];
        }
        // Use the cached union (fewer paths, no overlaps) instead of
        // raw fragments — equivalent with FillRule::NonZero.
        let new = {
            let cache = self.fragments_union_cache.lock().unwrap();
            match *cache {
                Some(ref u) => get_polygons_group_difference(polys, u),
                None => {
                    drop(cache);
                    get_polygons_group_difference(polys, &self.fragments)
                }
            }
        };
        if new.is_empty() {
            return new;
        }

        if self.does_any_overlap(&new) {
            // Local merge: union only the overlapping fragments with the
            // new polygons, keeping non-overlapping fragments untouched.
            self.apply_local_merge(&new);
        } else {
            // Fast path: no overlap, insert individually.
            for poly in &new {
                if poly.len() >= 3 {
                    let idx = self.fragments.len();
                    let bb = get_polygon_bounds(poly);
                    self.fragments.push(poly.clone());
                    self.grid.insert(idx, bb);
                }
            }
        }

        // Incrementally update the union cache with the newly added
        // polygons instead of recomputing from scratch.
        self.update_fragments_union(&new);

        // Full compaction when accumulation exceeds the threshold.
        const COMPACT_THRESHOLD: usize = 512;
        let total_vertices: usize =
            self.fragments.iter().map(|p| p.len()).sum();
        if total_vertices > COMPACT_THRESHOLD {
            self.fragments = get_polygons_union(&self.fragments);
            // After compaction the union is exact — update the cache to
            // match the compacted representation.
            *self.fragments_union_cache.lock().unwrap() =
                Some(self.fragments.clone());
            self.rebuild_grid();
        }

        self.clear_sweep_cache();
        new
    }

    /// Return a unioned, simplified snapshot of the current outer
    /// boundary, clipped to the stock.
    ///
    /// Uses the incrementally-maintained `fragments_union_cache` to
    /// avoid re-unioning all fragments from scratch.
    #[prof]
    pub fn frontier(
        &self,
        region: &StockRegion,
        simplify_tol: f64,
    ) -> Vec<Polygon> {
        let unioned = {
            let cache = self.fragments_union_cache.lock().unwrap();
            match *cache {
                Some(ref u) => u.clone(),
                None => {
                    drop(cache);
                    let u = get_polygons_union(&self.fragments);
                    *self.fragments_union_cache.lock().unwrap() =
                        Some(u.clone());
                    u
                }
            }
        };
        let stock = self.stock(region);
        let clipped = if stock.is_empty() {
            unioned
        } else {
            get_polygons_group_intersection(&unioned, &stock)
        };
        clipped
            .into_iter()
            .filter_map(|p| {
                let simplified = simplify_polyline(&p, simplify_tol);
                if simplified.len() < 3 {
                    None
                } else {
                    Some(simplified)
                }
            })
            .filter(|p| get_polygon_signed_area(p).abs() >= 0.01)
            .collect()
    }

    /// Expand the current frontier by `step_over`, clip to the
    /// tool-centre envelope, subtract already-cleared space, and
    /// return the resulting "bites".
    #[prof]
    pub fn bites(
        &self,
        region: &StockRegion,
        step_over: f64,
        tool_radius: f64,
        simplify_tol: f64,
    ) -> Vec<Polygon> {
        let f = self.frontier(region, simplify_tol);
        if f.is_empty() {
            return vec![];
        }
        let expanded: Vec<Polygon> = f
            .iter()
            .flat_map(|p| offset_polygon(p, step_over, JoinStyle::Round))
            .collect();
        if expanded.is_empty() {
            return vec![];
        }
        // Use the cached union (fewer paths, no overlaps) instead of
        // raw fragments for the Clipper difference — equivalent result
        // with FillRule::NonZero.
        let material = {
            let cache = self.fragments_union_cache.lock().unwrap();
            match *cache {
                Some(ref u) => get_polygons_group_difference(&expanded, u),
                None => {
                    drop(cache);
                    get_polygons_group_difference(&expanded, &self.fragments)
                }
            }
        };
        let envelope = self.envelope(region, tool_radius);
        get_polygons_group_intersection(&material, &envelope)
    }

    // ── Batched step expansion ──

    /// Begin buffering single‑segment expansions.
    ///
    /// Subsequent calls to [`expand_batched`](Self::expand_batched)
    /// will be queued without a union.  Call
    /// [`commit_step_batch`](Self::commit_step_batch) to union all queued
    /// sweeps with the stored fragments in a single pass.
    ///
    /// Calling this while a batch is already active is a no‑op.
    #[prof]
    pub fn begin_batch(&mut self) {
        self.batch_active = true;
    }

    /// Queue a segment `(prev → next)` with a disk of `radius`.
    ///
    /// The segment endpoint is stored in an internal path buffer.
    /// Does **not** perform a union until [`commit_batch`](Self::commit_batch)
    /// is called.
    ///
    /// # Panics
    /// Panics if `begin_batch()` was not called first.
    #[prof]
    pub fn expand_batched(&mut self, prev: Point, next: Point, radius: f64) {
        assert!(
            self.batch_active,
            "expand_batched called without begin_batch"
        );
        if radius < 1e-12 {
            return;
        }
        if self.batch_path.is_empty() {
            self.batch_path.push(prev);
            self.batch_radius = radius;
        }
        self.batch_path.push(next);
    }

    /// Build a single swept polygon from the accumulated path and union it
    /// with the stored fragments, then rebuild the spatial grid once.
    ///
    /// After this call the batch is closed (the caller may start a new one).
    #[prof]
    pub fn commit_batch(&mut self) {
        if !self.batch_active || self.batch_path.is_empty() {
            self.batch_active = false;
            return;
        }
        let radius = self.batch_radius;
        let path = std::mem::take(&mut self.batch_path);
        let swept = get_polyline_swept_polygon(&path, radius);
        self.union_swept(swept);
        self.batch_active = false;
    }

    /// Build a single swept polygon from the accumulated path and merge it
    /// only with nearby overlapping fragments (local union).
    ///
    /// After this call the batch is closed (the caller may start a new one).
    #[prof]
    pub fn commit_batch_local(&mut self) {
        if !self.batch_active || self.batch_path.is_empty() {
            self.batch_active = false;
            return;
        }
        let radius = self.batch_radius;
        let path = std::mem::take(&mut self.batch_path);
        let swept = get_polyline_swept_polygon(&path, radius);
        self.apply_local_merge(&swept);
        self.update_fragments_union(&swept);
        self.batch_active = false;
    }

    /// Local union: merge `swept` polygons only with fragments whose bbox
    /// overlaps them.
    #[prof]
    fn apply_local_merge(&mut self, swept: &[Polygon]) {
        if swept.is_empty() || swept.iter().all(|p| p.len() < 3) {
            return;
        }
        let mut to_merge: Vec<Polygon> =
            swept.iter().filter(|p| p.len() >= 3).cloned().collect();
        let mut removed: HashSet<usize> = HashSet::new();

        for _cascade in 0..2 {
            if to_merge.is_empty() {
                break;
            }
            let bbox = to_merge
                .iter()
                .map(get_polygon_bounds)
                .reduce(|a, b| {
                    Rect::new(
                        a.min.x.min(b.min.x),
                        a.min.y.min(b.min.y),
                        a.max.x.max(b.max.x),
                        a.max.y.max(b.max.y),
                    )
                })
                .unwrap();
            let margin = self.cell_size;
            let qbox = Rect::new(
                bbox.min.x - margin,
                bbox.min.y - margin,
                bbox.max.x + margin,
                bbox.max.y + margin,
            );
            let candidates: Vec<usize> = self
                .grid
                .query(qbox)
                .into_iter()
                .filter(|i| !removed.contains(i))
                .collect();
            if candidates.is_empty() {
                break;
            }
            for &ci in &candidates {
                removed.insert(ci);
                to_merge.push(self.fragments[ci].clone());
            }
            let merged = get_polygons_union(&to_merge);
            to_merge = merged;
        }

        // Remove old fragments in descending index order so swap_remove
        // never touches an already-processed position.
        let mut sorted: Vec<usize> = removed.into_iter().collect();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        for &fi in &sorted {
            self.fragments.swap_remove(fi);
        }
        // Add merged results.
        for poly in to_merge {
            if poly.len() >= 3 {
                self.fragments.push(poly);
            }
        }
        // NOTE: union cache is updated by the caller (cut_fast), not here,
        // to avoid double-updating on the overlap branch.
        self.clear_sweep_cache();
        self.rebuild_grid();
    }

    /// When total vertex count exceeds `threshold`, compact fragments by
    /// replacing them with the simplified frontier.
    #[prof]
    pub fn compact_if_needed(&mut self, region: &StockRegion, tol: f64) {
        self.compact_if_needed_threshold(region, tol, 75)
    }

    /// Like [`compact_if_needed`](Self::compact_if_needed) but with an
    /// explicit vertex-count threshold.
    #[prof]
    pub fn compact_if_needed_threshold(
        &mut self,
        region: &StockRegion,
        tol: f64,
        threshold: usize,
    ) {
        let total: usize = self.fragments.iter().map(|p| p.len()).sum();
        if total < threshold {
            return;
        }
        let frontier = self.frontier(region, tol);
        self.replace_fragments(frontier);
    }

    /// True when any polygon in `polys` overlaps an existing fragment.
    #[prof]
    fn does_any_overlap(&self, polys: &[Polygon]) -> bool {
        for poly in polys {
            if poly.len() < 3 {
                continue;
            }
            let bb = get_polygon_bounds(poly);
            if !self.grid.query(bb).is_empty() {
                return true;
            }
        }
        false
    }

    #[prof]
    fn rebuild_grid(&mut self) {
        self.grid = SpatialGrid::new(self.cell_size);
        for (idx, poly) in self.fragments.iter().enumerate() {
            self.grid.insert(idx, get_polygon_bounds(poly));
        }
    }

    /// Evaluate engagement at `center` using the signed distance to this
    /// cleared area's boundary.
    pub fn get_point_engagement(
        &self,
        center: Point,
        radius: f64,
    ) -> Engagement {
        crate::geo::algo::engagement::get_point_engagement(
            center,
            radius,
            &self.fragments,
        )
    }

    /// Compute angular engagement by exact circle–polygon intersection.
    ///
    /// Creates a disk polygon at `center` with `radius`, intersects it
    /// with all nearby cleared fragments, and returns the uncleared
    /// angular extent in `[0, 2π]`.
    #[prof]
    pub fn get_angular_engagement(&self, center: Point, radius: f64) -> f64 {
        let bb = Rect::new(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        );
        let nearby: Vec<Polygon> =
            self.query_window(bb).into_iter().cloned().collect();
        crate::geo::algo::engagement::get_angular_engagement(
            center, radius, &nearby,
        )
    }

    /// Compute the **incremental cut area** when the tool moves from
    /// `c1` to `c2`: the area inside the disk at `c2` that is NOT
    /// inside the disk at `c1` and NOT already cleared.
    ///
    /// This is the amount of *fresh* material the tool encounters by
    /// stepping forward — the metric used for engagement calculation.
    /// Unlike [`get_angular_engagement`](Self::get_angular_engagement) (which
    /// measures total overlap), this naturally prevents runaway
    /// outward drift because the crescent area is bounded by the step
    /// length.
    #[prof]
    pub fn cut_area(&self, c1: Point, c2: Point, radius: f64) -> f64 {
        self.cut_area_split(c1, c2, radius).0
    }

    /// Like [`cut_area`](Self::cut_area) but also returns the **left**
    /// portion — the area of the increment lying to the left of the
    /// step vector `c1 → c2` in the rotated frame.
    ///
    /// `right = total − left`.  This directional split lets the
    /// adaptive stepper detect when fresh material exists on both
    /// sides of the tool (a "breakthrough" between two cleared
    /// regions) and prefer the side matching `cut_direction`.
    #[prof]
    pub fn cut_area_split(
        &self,
        c1: Point,
        c2: Point,
        radius: f64,
    ) -> (f64, f64) {
        // Multi-entry LRU cache: the stepper probes 6-7 different
        // angles per step, and adjacent steps often revisit nearby
        // angles.  A small LRU catches both repeated hits and
        // cross-step overlaps.
        {
            let mut cache = self.sweep_cache.lock().unwrap();
            if let Some((total, left)) = cache.get(c1, c2, radius) {
                return (total, left);
            }
        }

        // Use nearby fragment cache: within a step, c1 is fixed and
        // fragments don't change — only the angle (and thus c2) varies.
        // The query_window call returns the same fragment set for all
        // angles because the step length is small relative to the query
        // window.
        let nearby = {
            let cache = self.nearby_cache.lock().unwrap();
            if let Some(ref c) = *cache {
                if c.c1 == c1 && c.radius == radius {
                    c.nearby.clone()
                } else {
                    drop(cache);
                    self.compute_and_cache_nearby(c1, radius)
                }
            } else {
                drop(cache);
                self.compute_and_cache_nearby(c1, radius)
            }
        };

        let (total, left) = crescent::cut_area(c1, c2, radius, &nearby, &[]);

        self.sweep_cache
            .lock()
            .unwrap()
            .insert(c1, c2, radius, total, left);

        (total, left)
    }

    /// Compute nearby fragments for `cut_area_split` and cache the result.
    #[inline]
    fn compute_and_cache_nearby(&self, c1: Point, radius: f64) -> Vec<Polygon> {
        let bb = Rect::new(
            c1.x - radius,
            c1.y - radius,
            c1.x + radius,
            c1.y + radius,
        );
        let nearby: Vec<Polygon> =
            self.query_window(bb).into_iter().cloned().collect();
        *self.nearby_cache.lock().unwrap() = Some(NearbyCache {
            c1,
            radius,
            nearby: nearby.clone(),
        });
        nearby
    }

    /// Evaluate engagement along a polyline for post-hoc analysis.
    pub fn path_engagement(
        &self,
        path: &[Point],
        radius: f64,
    ) -> Vec<Engagement> {
        path.iter()
            .map(|&p| self.get_point_engagement(p, radius))
            .collect()
    }
}

impl Clone for ClearedArea {
    fn clone(&self) -> Self {
        let mut ca = ClearedArea {
            fragments: self.fragments.clone(),
            grid: SpatialGrid::new(self.cell_size),
            cell_size: self.cell_size,
            batch_path: self.batch_path.clone(),
            batch_radius: self.batch_radius,
            batch_active: self.batch_active,
            sweep_cache: Mutex::new(SweepCache::EMPTY),
            nearby_cache: Mutex::new(None),
            fragments_union_cache: Mutex::new(None),
            remaining_area_cache: Mutex::new(None),
            actionable_cache: Mutex::new(None),
            frag_version: AtomicUsize::new(
                self.frag_version.load(Ordering::Relaxed),
            ),
            envelope_cache: Mutex::new(None),
        };
        ca.rebuild_grid();
        ca
    }
}

impl std::fmt::Debug for ClearedArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClearedArea")
            .field("fragments", &self.fragments.len())
            .field("cell_size", &self.cell_size)
            .field("batch_active", &self.batch_active)
            .finish()
    }
}

impl Default for ClearedArea {
    fn default() -> Self {
        Self::new()
    }
}

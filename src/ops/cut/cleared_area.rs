use std::collections::HashSet;
use std::sync::Mutex;

use prof_macros::prof;

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
use crate::ops::cut::crescent;
use crate::types::{Point, Polygon, Rect};

/// Single-entry cache for [`ClearedArea::cut_area_split`].
///
/// During a step evaluation the same `(c1, c2, radius)` triple is
/// queried many times (once the angle converges).  Because fragments
/// are immutable between commits, a single-entry cache avoids
/// redundant `prepare_sweep` + `sweep_area` calls.
struct SweepCache {
    c1: Point,
    c2: Point,
    radius: f64,
    total: f64,
    left: f64,
}

pub struct ClearedArea {
    /// Stock pocket outline (CCW).
    pub(crate) boundary: Polygon,
    /// Stock islands / holes (CW).
    pub(crate) islands: Vec<Polygon>,
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
    /// Single-entry result cache for [`Self::cut_area_split`].
    /// Cleared whenever fragments are mutated (see
    /// [`Self::clear_sweep_cache`]).
    sweep_cache: Mutex<Option<SweepCache>>,
    /// Cached `get_polygons_union(&self.fragments)` for
    /// [`Self::actionable_remaining`].  Cleared when fragments change.
    fragments_union_cache: Mutex<Option<Vec<Polygon>>>,
}

impl ClearedArea {
    // ── Construction ───────────────────────────────────────────────

    /// Create an empty cleared area inside the given stock
    /// (`boundary` = CCW pocket outline, `islands` = CW holes).
    pub fn new(boundary: &Polygon, islands: &[Polygon]) -> Self {
        let cell_size = 10.0;
        ClearedArea {
            boundary: boundary.clone(),
            islands: islands.to_vec(),
            fragments: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            batch_path: Vec::new(),
            batch_radius: 0.0,
            batch_active: false,
            sweep_cache: Mutex::new(None),
            fragments_union_cache: Mutex::new(None),
        }
    }

    /// Create a cleared area pre-seeded with `initial` polygons inside
    /// the given stock.  `initial` is **not** clipped.
    #[prof]
    pub fn from_polygons(
        initial: &[Polygon],
        boundary: &Polygon,
        islands: &[Polygon],
    ) -> Self {
        let cell_size = 10.0;
        let mut ca = ClearedArea {
            boundary: boundary.clone(),
            islands: islands.to_vec(),
            fragments: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            batch_path: Vec::new(),
            batch_radius: 0.0,
            batch_active: false,
            sweep_cache: Mutex::new(None),
            fragments_union_cache: Mutex::new(None),
        };
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
    fn stock(&self) -> Vec<Polygon> {
        if self.boundary.len() < 3 {
            return vec![];
        }
        if self.islands.is_empty() {
            vec![self.boundary.clone()]
        } else {
            get_polygons_group_difference(
                std::slice::from_ref(&self.boundary),
                &self.islands,
            )
        }
    }

    /// The tool-centre envelope (inset of boundary by `tool_radius`,
    /// minus islands).
    #[prof]
    pub fn envelope(&self, tool_radius: f64) -> Vec<Polygon> {
        compute_inset_region(&self.boundary, tool_radius, &self.islands).0
    }

    // ── Mutation ───────────────────────────────────────────────────

    /// Clear the single-entry sweep cache (called whenever fragments
    /// are modified).
    #[inline]
    fn clear_sweep_cache(&self) {
        *self.sweep_cache.lock().unwrap() = None;
        *self.fragments_union_cache.lock().unwrap() = None;
    }

    /// Replace all stored fragments with a new set (e.g., the new frontier
    /// after a wavefront advance).  This is O(m) in the new fragment count
    /// and avoids the O(n·m) accumulation of
    /// [`cut`](Self::cut).
    #[prof]
    pub fn replace_fragments(&mut self, fragments: Vec<Polygon>) {
        self.fragments = fragments;
        self.clear_sweep_cache();
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
    /// Polygons below 0.5 mm² are dropped as Clipper2 numerical artifacts.
    #[prof]
    pub fn remaining(&self) -> Vec<Polygon> {
        let stock = self.stock();
        if self.fragments.is_empty() {
            return stock;
        }
        if stock.is_empty() {
            return vec![];
        }
        let result = get_polygons_group_difference(&stock, &self.fragments);
        result
            .into_iter()
            .filter(|p| p.len() >= 3 && get_polygon_signed_area(p).abs() >= 0.5)
            .collect()
    }

    /// Area of uncut material remaining in the pocket.
    ///
    /// Computed as stock area minus the signed area of fragments
    /// intersected with the stock.  This avoids depending on the
    /// flat-path output of the difference operation (which can
    /// produce orphan CCW outers around islands).
    #[prof]
    pub fn remaining_area(&self) -> f64 {
        let stock = self.stock();
        if stock.is_empty() {
            return 0.0;
        }
        if self.fragments.is_empty() {
            return stock.iter().map(|p| get_polygon_signed_area(p)).sum();
        }
        let stock_area: f64 =
            stock.iter().map(|p| get_polygon_signed_area(p)).sum();
        let in_stock = get_polygons_group_intersection(&stock, &self.fragments);
        let cleared: f64 =
            in_stock.iter().map(|p| get_polygon_signed_area(p)).sum();
        (stock_area - cleared).max(0.0)
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
    pub fn actionable_remaining(&self, inset_distance: f64) -> f64 {
        let region =
            compute_inset_region(&self.boundary, inset_distance, &self.islands)
                .0;
        if region.is_empty() {
            return 0.0;
        }
        if self.fragments.is_empty() {
            // Region is already clipped to islands; outer rings
            // (CCW positive) and hole rings (CW negative) sum to the
            // net enclosed area.
            return region
                .iter()
                .map(|p| get_polygon_signed_area(p))
                .sum::<f64>()
                .max(0.0);
        }
        let unclipped = {
            let mut cache = self.fragments_union_cache.lock().unwrap();
            let unioned = cache
                .get_or_insert_with(|| get_polygons_union(&self.fragments));
            get_polygons_group_difference(&region, unioned)
        };
        // Clipper2 returns the difference as a bundle: outer rings
        // (positive signed area) and holes (negative signed area).
        // Summing signed areas gives correct net enclosed area.
        // Filter sub-resolution artefacts (< 0.5 mm²) the same way
        // `remaining()` does so the two metrics are comparable.
        let total: f64 = unclipped
            .into_iter()
            .filter(|p| p.len() >= 3 && get_polygon_signed_area(p).abs() >= 0.5)
            .map(|p| get_polygon_signed_area(&p))
            .sum();
        total.max(0.0)
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
        self.clear_sweep_cache();
        self.rebuild_grid();
    }

    /// Add polygons and return only the newly-added portion (input minus
    /// already-cleared area).  When none of the inputs overlap any existing
    /// fragment the union is skipped and they are appended directly for
    /// better performance.
    #[prof]
    pub fn cut_fast(&mut self, polys: &[Polygon]) -> Vec<Polygon> {
        if polys.is_empty() {
            return vec![];
        }
        let new = get_polygons_group_difference(polys, &self.fragments);
        if new.is_empty() {
            return new;
        }
        if self.does_any_overlap(&new) {
            let mut all = self.fragments.clone();
            all.extend(new.iter().cloned());
            self.fragments = get_polygons_union(&all);
            self.rebuild_grid();
        } else {
            for poly in &new {
                if poly.len() >= 3 {
                    let idx = self.fragments.len();
                    let bb = get_polygon_bounds(poly);
                    self.fragments.push(poly.clone());
                    self.grid.insert(idx, bb);
                }
            }
        }
        self.clear_sweep_cache();
        new
    }

    /// Return a unioned, simplified snapshot of the current outer
    /// boundary, clipped to the stock.
    #[prof]
    pub fn frontier(&self, simplify_tol: f64) -> Vec<Polygon> {
        let unioned = get_polygons_union(&self.fragments);
        let stock = self.stock();
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
            .filter(|p| get_polygon_signed_area(p).abs() >= 0.5)
            .collect()
    }

    /// Expand the current frontier by `step_over`, clip to the
    /// tool-centre envelope, subtract already-cleared space, and
    /// return the resulting "bites".
    #[prof]
    pub fn bites(
        &self,
        step_over: f64,
        tool_radius: f64,
        simplify_tol: f64,
    ) -> Vec<Polygon> {
        let f = self.frontier(simplify_tol);
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
        let material =
            get_polygons_group_difference(&expanded, &self.fragments);
        let envelope = self.envelope(tool_radius);
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
        self.clear_sweep_cache();
        self.rebuild_grid();
    }

    /// When total vertex count exceeds `threshold`, compact fragments by
    /// replacing them with the simplified frontier.
    #[prof]
    pub fn compact_if_needed(&mut self, tol: f64) {
        self.compact_if_needed_threshold(tol, 75)
    }

    /// Like [`compact_if_needed`](Self::compact_if_needed) but with an
    /// explicit vertex-count threshold.
    #[prof]
    pub fn compact_if_needed_threshold(&mut self, tol: f64, threshold: usize) {
        let total: usize = self.fragments.iter().map(|p| p.len()).sum();
        if total < threshold {
            return;
        }
        let frontier = self.frontier(tol);
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
        // Single-entry cache: when the stepper converges on the same
        // angle, repeated iterations (e.g. 8-19 of 20) query the same
        // (c1, c2, radius) triple.  Fragments are immutable between
        // commits, so the cached result is valid.
        {
            let cache = self.sweep_cache.lock().unwrap();
            if let Some(ref c) = *cache {
                if c.c1 == c1 && c.c2 == c2 && c.radius == radius {
                    return (c.total, c.left);
                }
            }
        }

        let bb = Rect::new(
            c2.x - radius,
            c2.y - radius,
            c2.x + radius,
            c2.y + radius,
        );
        let nearby: Vec<Polygon> =
            self.query_window(bb).into_iter().cloned().collect();
        let (total, left) = crescent::cut_area(c1, c2, radius, &nearby, &[]);

        *self.sweep_cache.lock().unwrap() = Some(SweepCache {
            c1,
            c2,
            radius,
            total,
            left,
        });

        (total, left)
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

impl Default for ClearedArea {
    fn default() -> Self {
        Self::new(&Polygon::new(), &[])
    }
}

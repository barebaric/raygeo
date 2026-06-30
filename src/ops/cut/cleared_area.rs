use std::collections::HashSet;

use prof_macros::prof;

use crate::geo::algo::engagement::Engagement;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::algo::simplify::simplify_polyline;
use crate::geo::algo::spatial_grid2d::SpatialGrid;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_bounds, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_union,
    get_segment_swept_polygon, offset_polygon, JoinStyle,
};
use crate::ops::cut::crescent;
use crate::types::{Point, Polygon, Rect};

pub struct ClearedArea {
    /// Stock pocket outline (CCW).
    pub(crate) boundary: Polygon,
    /// Stock islands / holes (CW).
    pub(crate) islands: Vec<Polygon>,
    pub(crate) fragments: Vec<Polygon>,
    pub(crate) grid: SpatialGrid,
    pub(crate) cell_size: f64,
    // ── Batched step expansion ──
    /// Buffer of swept polygons accumulated while a batch is open.
    pub(crate) batch_buffer: Vec<Polygon>,
    /// True when `begin_batch()` has been called.
    pub(crate) batch_active: bool,
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
            batch_buffer: Vec::new(),
            batch_active: false,
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
            batch_buffer: Vec::new(),
            batch_active: false,
        };
        for poly in initial {
            if poly.len() >= 3 {
                let idx = ca.fragments.len();
                ca.fragments.push(poly.clone());
                ca.grid.insert(idx, get_polygon_bounds(poly));
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

    /// Replace all stored fragments with a new set (e.g., the new frontier
    /// after a wavefront advance).  This is O(m) in the new fragment count
    /// and avoids the O(n·m) accumulation of
    /// [`cut`](Self::cut).
    #[prof]
    pub fn replace_fragments(&mut self, fragments: Vec<Polygon>) {
        self.fragments = fragments;
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

        let mut all_polys: Vec<Polygon> = self.fragments.clone();
        all_polys.extend(swept_polys);
        let merged = get_polygons_union(&all_polys);

        self.fragments = merged;
        self.rebuild_grid();
    }

    #[prof]
    pub fn expand_step(&mut self, prev: Point, next: Point, radius: f64) {
        let swept = get_segment_swept_polygon(prev, next, radius);
        let mut all_polys = self.fragments.clone();
        all_polys.extend(swept);
        let merged = get_polygons_union(&all_polys);
        self.fragments = merged;
        self.rebuild_grid();
    }

    #[prof]
    pub fn query_window(&self, bbox: Rect) -> Vec<&Polygon> {
        let indices = self.grid.query(bbox);
        let mut result: Vec<&Polygon> = indices
            .iter()
            .filter_map(|&idx| self.fragments.get(idx))
            .collect();
        result.sort_by(|a, b| {
            a.first()
                .and_then(|pa| {
                    b.first().map(|pb| {
                        pa.x.partial_cmp(&pb.x)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                })
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    /// Return the uncut stock: stock ∖ fragments.
    #[prof]
    pub fn remaining(&self) -> Vec<Polygon> {
        let stock = self.stock();
        if self.fragments.is_empty() {
            return stock;
        }
        if stock.is_empty() {
            return vec![];
        }
        get_polygons_group_difference(&stock, &self.fragments)
    }

    #[prof]
    pub fn total_area(&self) -> f64 {
        self.fragments.iter().map(get_polygon_area).sum()
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
        if self.any_overlap(&new) {
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
    /// The swept polygon is stored in an internal buffer.  Does **not**
    /// perform a union until [`commit_batch`](Self::commit_batch)
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
        let swept = get_segment_swept_polygon(prev, next, radius);
        self.batch_buffer.extend(swept);
    }

    /// Union all buffered sweeps with the stored fragments in a single pass,
    /// then rebuild the spatial grid once.
    ///
    /// After this call the batch is closed (the caller may start a new one).
    #[prof]
    pub fn commit_batch(&mut self) {
        if !self.batch_active || self.batch_buffer.is_empty() {
            self.batch_active = false;
            return;
        }
        let buf = std::mem::take(&mut self.batch_buffer);
        let mut all_polys = self.fragments.clone();
        all_polys.extend(buf);
        self.fragments = get_polygons_union(&all_polys);
        self.rebuild_grid();
        self.batch_active = false;
    }

    /// Union only the buffered sweeps with nearby overlapping fragments,
    /// using the spatial grid to avoid touching distant fragments.
    ///
    /// After this call the batch is closed (the caller may start a new one).
    #[prof]
    pub fn commit_batch_local(&mut self) {
        if !self.batch_active || self.batch_buffer.is_empty() {
            self.batch_active = false;
            return;
        }
        let buf = std::mem::take(&mut self.batch_buffer);
        self.apply_local_merge(&buf);
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
        self.rebuild_grid();
    }

    /// When total vertex count exceeds `threshold`, compact fragments by
    /// replacing them with the simplified frontier.
    #[prof]
    pub fn compact_if_needed(&mut self, tol: f64) {
        self.compact_if_needed_threshold(tol, 50_000)
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
    fn any_overlap(&self, polys: &[Polygon]) -> bool {
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
    pub fn point_engagement(&self, center: Point, radius: f64) -> Engagement {
        crate::geo::algo::engagement::point_engagement(
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
    pub fn angular_engagement(&self, center: Point, radius: f64) -> f64 {
        let bb = Rect::new(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        );
        let nearby: Vec<Polygon> =
            self.query_window(bb).into_iter().cloned().collect();
        crate::geo::algo::engagement::angular_engagement(
            center, radius, &nearby,
        )
    }

    /// Compute the **incremental cut area** when the tool moves from
    /// `c1` to `c2`: the area inside the disk at `c2` that is NOT
    /// inside the disk at `c1` and NOT already cleared.
    ///
    /// This is the amount of *fresh* material the tool encounters by
    /// stepping forward — the metric used for engagement calculation.
    /// Unlike [`angular_engagement`](Self::angular_engagement) (which
    /// measures total overlap), this naturally prevents runaway
    /// outward drift because the crescent area is bounded by the step
    /// length.
    #[prof]
    pub fn cut_area(&self, c1: Point, c2: Point, radius: f64) -> f64 {
        self.cut_area_split(c1, c2, radius).0
    }

    /// Compute the incremental cut area and split it into climb /
    /// conventional components.
    #[prof]
    pub fn cut_area_milling(
        &self,
        c1: Point,
        c2: Point,
        radius: f64,
    ) -> (f64, f64) {
        self.cut_area_split(c1, c2, radius)
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
        let bb = Rect::new(
            c2.x - radius,
            c2.y - radius,
            c2.x + radius,
            c2.y + radius,
        );
        let nearby: Vec<Polygon> =
            self.query_window(bb).into_iter().cloned().collect();
        crescent::cut_area(c1, c2, radius, &nearby, &[])
    }

    /// Evaluate engagement along a polyline for post-hoc analysis.
    pub fn path_engagement(
        &self,
        path: &[Point],
        radius: f64,
    ) -> Vec<Engagement> {
        path.iter()
            .map(|&p| self.point_engagement(p, radius))
            .collect()
    }
}

impl Default for ClearedArea {
    fn default() -> Self {
        Self::new(&Polygon::new(), &[])
    }
}

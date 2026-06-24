use prof_macros::prof;

use crate::geo::algo::engagement::compute_engagement;
use crate::geo::algo::engagement::Engagement;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::algo::simplify::simplify_polyline_3d;
use crate::geo::algo::spatial_grid2d::SpatialGrid;
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::geo::shape::polygon::get_polygon_area;
use crate::geo::shape::polygon::get_polygon_bounds;
use crate::geo::shape::polygon::get_polygon_centroid;
use crate::geo::shape::polygon::get_polygons_closest_point;
use crate::geo::shape::polygon::get_polygons_group_difference;
use crate::geo::shape::polygon::get_polygons_group_intersection;
use crate::geo::shape::polygon::get_polygons_union;
use crate::geo::shape::polygon::get_segment_swept_polygon;
use crate::geo::shape::polygon::offset_polygon;
use crate::geo::shape::polygon::JoinStyle;
use crate::types::{Point, Point3D, Polygon, Rect};

pub struct ClearedArea {
    fragments: Vec<Polygon>,
    bboxes: Vec<Rect>,
    grid: SpatialGrid,
    cell_size: f64,
    /// Flat list of all edges across all fragments, indexed by `edge_grid`.
    edge_pts: Vec<(Point, Point)>,
    /// Spatial grid over individual fragment edges for fast nearest-edge
    /// queries from arbitrary query points.
    edge_grid: SpatialGrid,
    /// Cell size for the edge-level spatial grid (finer than the polygon
    /// grid so that thin crescents are captured).
    edge_cell_size: f64,
    // ── Batched step expansion ──
    /// Buffer of swept polygons accumulated while a batch is open.
    batch_buffer: Vec<Polygon>,
    /// True when `begin_step_batch()` has been called.
    batch_active: bool,
}

/// A resume point found on the cleared-area frontier.
#[derive(Debug, Clone)]
pub struct ResumePoint {
    /// Position on the frontier.
    pub pos: Point,
    /// Outward-normal heading at the resume position (radians).
    pub heading: f64,
    /// Travel polyline through cleared territory, routed by the MAT.
    pub link_path: Vec<Point>,
}

// ── Stepper types ──

/// Options controlling the stepping solver.
#[derive(Clone, Debug)]
pub struct StepperOptions {
    /// Disk radius (mm).
    pub radius: f64,
    /// Forward distance per step (mm).  Typical value: `radius × 0.2`.
    pub step_length: f64,
    /// Target overlap angle (radians).  Derived from the advance ratio:
    /// `target_engagement = 2·π − 2·acos(advance / radius)`.
    /// In `[0, 2π]`.
    pub target_engagement: f64,
    /// Solver tolerance on engagement angle (radians).  Default `0.01`.
    pub engagement_tol: f64,
    /// Maximum steering deflection per step (radians).  Default ~30°.
    pub max_deflection: f64,
    /// Maximum solver iterations per step.  Default `6` (usually converges
    /// in 2–3 on smooth geometry).
    pub max_solver_iters: usize,
}

impl Default for StepperOptions {
    fn default() -> Self {
        Self {
            radius: 3.0,
            step_length: 0.6,
            target_engagement: std::f64::consts::PI,
            engagement_tol: 0.01,
            max_deflection: std::f64::consts::FRAC_PI_6,
            max_solver_iters: 6,
        }
    }
}

/// Result of a single step.
#[derive(Debug, Clone)]
pub struct StepResult {
    /// New centre position.
    pub next: Point,
    /// Updated heading (radians).
    pub heading: f64,
    /// Measured overlap angle at `next`.
    pub engagement: Engagement,
    /// Solver iterations consumed.
    pub iters: usize,
    /// Termination status.
    pub status: StepStatus,
}

/// Status returned by a single step or a full segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    /// The step converged normally.
    Ok,
    /// The disk reached or crossed the domain boundary.
    BoundaryHit,
    /// No valid overlap can be found (disk is in open space or fully
    /// inside the cleared area).
    LostEngagement,
    /// The solver could not converge within the budget.
    NoConvergence,
}

/// Derive the target engagement angle from the advance ratio.
///
/// When `advance >= radius` the angle saturates at `2π` (full
/// overlap).  A typical advance is 10–40 % of radius,
/// giving engagement angles roughly 145°–205°.
pub fn target_engagement_from_advance(advance: f64, radius: f64) -> f64 {
    if advance <= 0.0 || radius <= 0.0 {
        return std::f64::consts::PI;
    }
    let ratio = (advance / radius).clamp(0.0, 1.0);
    2.0 * std::f64::consts::PI - 2.0 * (1.0 - ratio).acos()
}

/// Try to find a steering angle via 7-sample grid with interpolation.
fn try_bracket(
    heading: f64,
    opts: &StepperOptions,
    engagement_at: &dyn Fn(f64) -> f64,
) -> (f64, StepStatus, usize) {
    let target = opts.target_engagement;
    let max_def = opts.max_deflection;

    let f0 = engagement_at(heading) - target;

    let ratios = [-1.0, -0.6, -0.2, 0.0, 0.2, 0.6, 1.0];
    let mut samples: [(f64, f64); 7] = [(0.0, 0.0); 7];
    for (i, &r) in ratios.iter().enumerate() {
        let phi = heading + max_def * r;
        let err = if r == 0.0 {
            f0
        } else {
            engagement_at(phi) - target
        };
        samples[i] = (phi, err);
    }

    for i in 0..samples.len() - 1 {
        let (a, fa) = samples[i];
        let (b, fb) = samples[i + 1];
        if fa.is_finite() && fb.is_finite() && fa.signum() != fb.signum() {
            let t = -fa / (fb - fa);
            let root = a + t * (b - a);
            return (root, StepStatus::Ok, 2);
        }
    }

    let best = samples
        .iter()
        .min_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
        .unwrap();
    (best.0, StepStatus::Ok, samples.len())
}

impl ClearedArea {
    /// Perform one forward step.
    ///
    /// Starting from `pos` with the given `heading` (radians), propose candidate
    /// positions at `step_length` distance along trial deflection angles and
    /// solve for the heading that maintains the target engagement.
    pub fn step(
        &self,
        pos: Point,
        heading: f64,
        opts: &StepperOptions,
    ) -> StepResult {
        let cur_eng = self.point_engagement(pos, opts.radius);
        if cur_eng.angle < opts.target_engagement * 0.05 {
            return StepResult {
                next: pos,
                heading,
                engagement: cur_eng,
                iters: 0,
                status: StepStatus::LostEngagement,
            };
        }

        let engagement_at = |phi: f64| -> f64 {
            let dir = Point::new(phi.cos(), phi.sin());
            let candidate = pos + dir * opts.step_length;
            let eng = self.point_engagement(candidate, opts.radius);
            eng.angle
        };

        let (best_phi, step_status, iters) =
            try_bracket(heading, opts, &engagement_at);

        let mut step_len = opts.step_length;
        if step_status == StepStatus::Ok {
            let eng_at_best = engagement_at(best_phi);
            let cur_err = cur_eng.angle - opts.target_engagement;
            let best_err = eng_at_best - opts.target_engagement;
            if cur_err * best_err < 0.0 && cur_err.abs() > opts.engagement_tol {
                let t = cur_err / (cur_err - best_err);
                step_len *= t.clamp(0.25, 1.0);
            } else if best_err > opts.engagement_tol
                && cur_err.abs() <= opts.engagement_tol
            {
                let t = (opts.target_engagement - cur_eng.angle)
                    / (eng_at_best - cur_eng.angle);
                step_len *= t.clamp(0.25, 0.5);
            }
        }

        let dir = Point::new(best_phi.cos(), best_phi.sin());
        let next_pos = pos + dir * step_len;
        let eng = self.point_engagement(next_pos, opts.radius);

        StepResult {
            next: next_pos,
            heading: best_phi,
            engagement: eng,
            iters,
            status: step_status,
        }
    }

    /// Drive the disk forward calling [`step`] until a non‑`Ok` status or
    /// `max_steps` is reached.
    ///
    /// Returns the centre path and the final status.
    /// Does **not** modify the `ClearedArea` — the caller is responsible for
    /// committing swept polygons after the segment.
    pub fn run_segment(
        &self,
        start: Point,
        initial_heading: f64,
        opts: &StepperOptions,
        max_steps: usize,
    ) -> (Vec<Point>, StepStatus) {
        let mut path = Vec::with_capacity(max_steps.min(10000));
        path.push(start);

        let mut pos = start;
        let mut heading = initial_heading;

        for _ in 0..max_steps {
            let result = self.step(pos, heading, opts);
            match result.status {
                StepStatus::Ok => {
                    path.push(result.next);
                    pos = result.next;
                    heading = result.heading;
                }
                other => {
                    return (path, other);
                }
            }
        }

        (path, StepStatus::Ok)
    }
}

/// Internal: position + heading pair found on the frontier.
struct FrontierCandidate {
    pos: Point,
    heading: f64,
}

impl ClearedArea {
    fn insert_poly_edges(
        edge_pts: &mut Vec<(Point, Point)>,
        edge_grid: &mut SpatialGrid,
        poly: &[Point],
    ) {
        let m = poly.len();
        for i in 0..m {
            let j = (i + 1) % m;
            let a = poly[i];
            let b = poly[j];
            let idx = edge_pts.len();
            edge_pts.push((a, b));
            edge_grid.insert(
                idx,
                Rect::new(
                    a.x.min(b.x),
                    a.y.min(b.y),
                    a.x.max(b.x),
                    a.y.max(b.y),
                ),
            );
        }
    }

    pub fn new() -> Self {
        let cell_size = 10.0;
        let edge_cell_size = 5.0;
        ClearedArea {
            fragments: Vec::new(),
            bboxes: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            edge_pts: Vec::new(),
            edge_grid: SpatialGrid::new(edge_cell_size),
            edge_cell_size,
            batch_buffer: Vec::new(),
            batch_active: false,
        }
    }

    pub fn from_polygons(initial: &[Polygon]) -> Self {
        let cell_size = 10.0;
        let edge_cell_size = 5.0;
        let mut ca = ClearedArea {
            fragments: Vec::new(),
            bboxes: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            edge_pts: Vec::new(),
            edge_grid: SpatialGrid::new(edge_cell_size),
            edge_cell_size,
            batch_buffer: Vec::new(),
            batch_active: false,
        };
        for poly in initial {
            if poly.len() >= 3 {
                let idx = ca.fragments.len();
                ca.fragments.push(poly.clone());
                ca.bboxes.push(get_polygon_bounds(poly));
                ca.grid.insert(idx, get_polygon_bounds(poly));
            }
        }
        ca
    }

    /// Replace all stored fragments with a new set (e.g., the new frontier
    /// after a wavefront advance).  This is O(m) in the new fragment count
    /// and avoids the O(n·m) accumulation of
    /// [`add_cleared_polygons`](Self::add_cleared_polygons).
    pub fn replace_fragments(&mut self, fragments: Vec<Polygon>) {
        self.fragments = fragments;
        self.rebuild_grid();
    }

    /// Return the squared distance from `(x, y)` to the nearest edge of
    /// any cleared fragment.
    ///
    /// Uses the edge-level spatial grid so that only edges in nearby
    /// grid cells are checked.  Returns `f64::MAX` when there are no
    /// fragments **or** the point is too far from any edge to be found
    /// in the local query window.
    pub fn closest_boundary_distance_sq(&self, x: f64, y: f64) -> f64 {
        let cell_size = self.edge_cell_size;
        let mut qbox = Rect::new(x, y, x, y);
        qbox.min.x -= cell_size;
        qbox.min.y -= cell_size;
        qbox.max.x += cell_size;
        qbox.max.y += cell_size;
        let mut candidates = Vec::new();
        self.edge_grid.query_into(qbox, &mut candidates);
        if candidates.is_empty() {
            return f64::MAX;
        }
        let mut best_d2 = f64::MAX;
        for &ei in &candidates {
            let (a, b) = self.edge_pts[ei];
            let (_, _, d2) = get_line_segment_closest_point(a, b, x, y);
            if d2 < best_d2 {
                best_d2 = d2;
            }
        }
        best_d2
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
        use crate::geo::shape::polygon::is_point_in_polygon;

        let pt = Point::new(x, y);
        let inside = self
            .fragments
            .iter()
            .any(|frag| frag.len() >= 3 && is_point_in_polygon(pt, frag));

        let d = get_polygons_closest_point(&self.fragments, pt)
            .map(|(_, _, _, d2)| d2.sqrt())
            .unwrap_or(f64::MAX);

        if inside {
            -d.abs()
        } else {
            d
        }
    }

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

    pub fn expand_step(&mut self, prev: Point, next: Point, radius: f64) {
        let swept = get_segment_swept_polygon(prev, next, radius);
        let mut all_polys = self.fragments.clone();
        all_polys.extend(swept);
        let merged = get_polygons_union(&all_polys);
        self.fragments = merged;
        self.rebuild_grid();
    }

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

    pub fn remaining(&self, bounds: &[Polygon]) -> Vec<Polygon> {
        if self.fragments.is_empty() {
            return bounds.to_vec();
        }
        if bounds.is_empty() {
            return vec![];
        }
        get_polygons_group_difference(bounds, &self.fragments)
    }

    /// Compute the inset region of `boundary` by `radius` (excluding
    /// `obstacles`), then return the portions of that region not covered
    /// by stored fragments, together with the original obstacle polygons.
    pub fn remaining_in_inset(
        &self,
        boundary: &Polygon,
        obstacles: &[Polygon],
        radius: f64,
    ) -> Vec<Polygon> {
        let (inset_region, _) =
            compute_inset_region(boundary, radius, obstacles);
        let mut result = obstacles.to_vec();
        result.extend(get_polygons_group_difference(
            &inset_region,
            self.fragments(),
        ));
        result
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
    pub fn add_cleared_polygons(&mut self, polygons: &[Polygon]) {
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
    pub fn incorporate(&mut self, polys: &[Polygon]) -> Vec<Polygon> {
        if polys.is_empty() {
            return vec![];
        }
        let new = self.remaining(polys);
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
                    self.bboxes.push(bb);
                    self.grid.insert(idx, bb);
                    Self::insert_poly_edges(
                        &mut self.edge_pts,
                        &mut self.edge_grid,
                        poly,
                    );
                }
            }
        }
        new
    }

    /// Return a unioned, simplified snapshot of the current outer boundary.
    pub fn frontier(&self, simplify_tol: f64) -> Vec<Polygon> {
        let unioned = get_polygons_union(&self.fragments);
        unioned
            .into_iter()
            .filter_map(|p| {
                let pts3d: Vec<Point3D> =
                    p.iter().map(|q| Point3D::new(q.x, q.y, 0.0)).collect();
                let simplified = simplify_polyline_3d(&pts3d, simplify_tol);
                if simplified.len() < 3 {
                    None
                } else {
                    Some(
                        simplified
                            .iter()
                            .map(|q| Point::new(q.x, q.y))
                            .collect(),
                    )
                }
            })
            .collect()
    }

    /// Expand the current frontier outward by `step_over`, clip to
    /// `valid_area`, and return the newly-exposed region (the
    /// difference between the expanded frontier and the stored
    /// fragments).  Does NOT modify the stored fragments — the caller
    /// is responsible for calling
    /// [`absorb_frontier`](Self::absorb_frontier) afterwards.
    #[prof]
    pub fn compute_bites(
        &self,
        step_over: f64,
        valid_area: &[Polygon],
        simplify_tol: f64,
    ) -> Vec<Polygon> {
        if self.fragments.is_empty() {
            return vec![];
        }
        let simplify = |p: &Polygon| -> Option<Polygon> {
            if p.len() < 3 {
                return None;
            }
            let pts3d: Vec<Point3D> =
                p.iter().map(|q| Point3D::new(q.x, q.y, 0.0)).collect();
            let simplified = simplify_polyline_3d(&pts3d, simplify_tol);
            if simplified.len() < 3 {
                None
            } else {
                Some(simplified.iter().map(|q| Point::new(q.x, q.y)).collect())
            }
        };

        let unioned = get_polygons_union(&self.fragments);
        let offset_src: Vec<Polygon> =
            unioned.iter().filter_map(simplify).collect();
        if offset_src.is_empty() {
            return vec![];
        }
        let expanded: Vec<Polygon> = offset_src
            .iter()
            .flat_map(|p| offset_polygon(p, step_over, JoinStyle::Round))
            .collect();
        if expanded.is_empty() {
            return vec![];
        }
        let mut region =
            get_polygons_group_difference(&expanded, &self.fragments);
        region = get_polygons_group_intersection(&region, valid_area);
        region
    }

    /// Absorb a set of polygons (e.g. the output of
    /// [`compute_bites`](Self::compute_bites)) into the stored
    /// fragments by unioning them, without accumulating historical
    /// fragments.
    #[prof]
    pub fn absorb_frontier(&mut self, region: &[Polygon]) {
        if region.is_empty() {
            return;
        }
        self.fragments.extend(region.iter().cloned());
        let unioned = get_polygons_union(&self.fragments);
        self.replace_fragments(unioned);
    }

    /// Expand the current frontier by `step_over`, clip to `valid_area`,
    /// subtract already-cleared space, and return the resulting "bites".
    #[prof]
    pub fn bites(
        &self,
        step_over: f64,
        valid_area: &[Polygon],
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
        let material = self.remaining(&expanded);
        get_polygons_group_intersection(&material, valid_area)
    }

    /// Like [`bites`](Self::bites) but only returns the bites whose
    /// centroid lies within `max_angle` radians of the direction from
    /// this area's centre toward `target`.
    ///
    /// When the cleared area is empty the direction-filter is skipped
    /// and all bites are returned.
    pub fn bite_in_direction(
        &self,
        step_over: f64,
        valid_area: &[Polygon],
        simplify_tol: f64,
        target: Point,
        max_angle: f64,
    ) -> Vec<Polygon> {
        let all = self.bites(step_over, valid_area, simplify_tol);
        if all.is_empty()
            || self.fragments.is_empty()
            || max_angle >= std::f64::consts::PI
        {
            return all;
        }

        // Compute centre of current cleared area.
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut count = 0usize;
        for frag in &self.fragments {
            for p in frag {
                cx += p.x;
                cy += p.y;
                count += 1;
            }
        }
        let centre = Point::new(cx / count as f64, cy / count as f64);
        let dir = target - centre;
        let dir_len = dir.length();
        if dir_len < 1e-12 {
            return all;
        }
        let dir = dir / dir_len;

        let cos_max = max_angle.cos();

        all.into_iter()
            .filter(|bite| {
                let bc = get_polygon_centroid(bite);
                let to_bite = bc - centre;
                let len = to_bite.length();
                len <= step_over || dir.dot(to_bite / len) >= cos_max
            })
            .collect()
    }

    /// Return all passes of isotropic bites needed to fully clear the
    /// valid area.
    ///
    /// Each inner `Vec` is one pass (all bites generated from the same
    /// frontier).  Passes are ordered from the centre of the cleared area
    /// outward.  The cleared area is fully cleared after this call.
    ///
    /// Note: this method uses the older accumulation-based approach
    /// ([`bites`] + [`incorporate`]).  For better performance on large
    /// pockets, use [`compute_bites`](Self::compute_bites) /
    /// [`absorb_frontier`](Self::absorb_frontier) in a
    /// caller-managed loop.
    pub fn all_bites(
        &mut self,
        step_over: f64,
        valid_area: &[Polygon],
        simplify_tol: f64,
    ) -> Vec<Vec<Polygon>> {
        let mut passes = Vec::new();
        loop {
            let bites = self.bites(step_over, valid_area, simplify_tol);
            if bites.is_empty() {
                break;
            }
            passes.push(bites);
            self.incorporate(passes.last().unwrap());
        }
        passes
    }

    // ── Batched step expansion ──

    /// Begin buffering single‑segment expansions.
    ///
    /// Subsequent calls to [`expand_step_batched`](Self::expand_step_batched)
    /// will be queued without a union.  Call
    /// [`commit_step_batch`](Self::commit_step_batch) to union all queued
    /// sweeps with the stored fragments in a single pass.
    ///
    /// Calling this while a batch is already active is a no‑op.
    pub fn begin_step_batch(&mut self) {
        self.batch_active = true;
    }

    /// Queue a segment `(prev → next)` with a disk of `radius`.
    ///
    /// The swept polygon is stored in an internal buffer.  Does **not**
    /// perform a union until [`commit_step_batch`](Self::commit_step_batch)
    /// is called.
    ///
    /// # Panics
    /// Panics if `begin_step_batch()` was not called first.
    pub fn expand_step_batched(
        &mut self,
        prev: Point,
        next: Point,
        radius: f64,
    ) {
        assert!(
            self.batch_active,
            "expand_step_batched called without begin_step_batch"
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
    pub fn commit_step_batch(&mut self) {
        if !self.batch_active || self.batch_buffer.is_empty() {
            self.batch_active = false;
            return;
        }
        let mut all_polys = self.fragments.clone();
        all_polys.append(&mut self.batch_buffer);
        self.fragments = get_polygons_union(&all_polys);
        self.rebuild_grid();
        self.batch_active = false;
    }

    /// True when any polygon in `polys` overlaps an existing fragment.
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

    fn rebuild_grid(&mut self) {
        self.grid = SpatialGrid::new(self.cell_size);
        self.bboxes.clear();
        for (idx, poly) in self.fragments.iter().enumerate() {
            let bbox = get_polygon_bounds(poly);
            self.bboxes.push(bbox);
            self.grid.insert(idx, bbox);
        }

        self.edge_grid = SpatialGrid::new(self.edge_cell_size);
        self.edge_pts.clear();
        for poly in &self.fragments {
            if poly.len() >= 2 {
                Self::insert_poly_edges(
                    &mut self.edge_pts,
                    &mut self.edge_grid,
                    poly,
                );
            }
        }
    }

    /// Evaluate engagement at `center` using the signed distance to this
    /// cleared area's boundary.
    pub fn point_engagement(&self, center: Point, radius: f64) -> Engagement {
        let d = self.signed_boundary_distance(center.x, center.y);
        compute_engagement(d, radius)
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

    /// Walk the cleared-area frontier forward from a point near `end_pos`
    /// and return the first position where engagement ≥ `min_engagement`.
    ///
    /// The link path between `end_pos` and the resume position is routed
    /// through the cleared area via MAT for collision‑free travel.
    ///
    /// Returns `None` when no valid resume point is found (fully cleared
    /// or no MAT available).
    pub fn find_next_resume(
        &self,
        mat: &MedialAxis,
        end_pos: Point,
        radius: f64,
        min_engagement: f64,
    ) -> Option<ResumePoint> {
        if self.is_empty() {
            return None;
        }

        let frontier = self.frontier(0.5);
        if frontier.is_empty() {
            return None;
        }

        // Find the closest point on the frontier to end_pos
        // using the general polygon query.
        let (closest_poly_idx, _t, closest_pt, _d2) =
            get_polygons_closest_point(&frontier, end_pos)?;

        // Build a FrontierCandidate from the closest point.
        let poly = &frontier[closest_poly_idx];
        let heading = frontier_heading_at(closest_pt, poly);
        let start_candidate = FrontierCandidate {
            pos: closest_pt,
            heading,
        };

        // Walk the frontier forward, checking engagement.
        let walk_candidates = walk_frontier_forward(
            &start_candidate,
            &frontier,
            radius,
            min_engagement,
            self,
        );

        let resume_pt = walk_candidates.into_iter().next()?;

        let link = mat
            .path_between(end_pos, resume_pt.pos)
            .unwrap_or_else(|| vec![end_pos, resume_pt.pos]);

        Some(ResumePoint {
            pos: resume_pt.pos,
            heading: resume_pt.heading,
            link_path: link,
        })
    }
}

/// Walk the frontier polygon forward from `start`, checking engagement at
/// each vertex.  Returns candidates with engagement ≥ min_engagement.
fn walk_frontier_forward(
    start: &FrontierCandidate,
    frontier: &[Vec<Point>],
    radius: f64,
    min_engagement: f64,
    cleared: &ClearedArea,
) -> Vec<FrontierCandidate> {
    let mut candidates = Vec::new();

    for poly in frontier {
        if poly.len() < 3 {
            continue;
        }

        // Find the nearest vertex index in this polygon.
        let start_idx = poly.iter().enumerate().min_by(|(_, a), (_, b)| {
            a.distance_squared(start.pos)
                .partial_cmp(&b.distance_squared(start.pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let start_idx = match start_idx {
            Some((i, _)) => i,
            None => continue,
        };

        // Walk forward from start_idx (wrap around).
        let n = poly.len();
        for offset in 0..n {
            let idx = (start_idx + offset) % n;
            let pt = poly[idx];
            let eng = cleared.point_engagement(pt, radius);
            if eng.angle >= min_engagement {
                let heading = frontier_heading_at(pt, poly);
                candidates.push(FrontierCandidate { pos: pt, heading });
                break;
            }
        }
    }

    candidates
}

/// Estimate the outward-normal heading at a vertex of a frontier polygon.
fn frontier_heading_at(v: Point, poly: &[Point]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let idx = poly.iter().enumerate().min_by(|(_, a), (_, b)| {
        a.distance_squared(v)
            .partial_cmp(&b.distance_squared(v))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let idx = match idx {
        Some((i, _)) => i,
        None => return 0.0,
    };
    let n = poly.len();
    let prev = poly[(idx + n - 1) % n];
    let next = poly[(idx + 1) % n];
    let tangent = next - prev;
    if tangent.length_squared() < 1e-12 {
        return 0.0;
    }
    let tangent = tangent.normalize();
    let outward = Point::new(-tangent.y, tangent.x);
    outward.y.atan2(outward.x)
}

impl Default for ClearedArea {
    fn default() -> Self {
        Self::new()
    }
}

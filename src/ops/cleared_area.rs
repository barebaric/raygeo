use prof_macros::prof;

use crate::geo::algo::engagement::Engagement;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::simplify::simplify_polyline_3d;
use crate::geo::algo::spatial_grid2d::SpatialGrid;
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::geo::shape::polygon::get_polygon_area;
use crate::geo::shape::polygon::get_polygon_bounds;
use crate::geo::shape::polygon::get_polygon_signed_area;
use crate::geo::shape::polygon::get_polygons_closest_point;
use crate::geo::shape::polygon::get_polygons_group_difference;
use crate::geo::shape::polygon::get_polygons_group_intersection;
use crate::geo::shape::polygon::get_polygons_union;
use crate::geo::shape::polygon::get_segment_swept_polygon;
use crate::geo::shape::polygon::is_point_in_polygon;
use crate::geo::shape::polygon::offset_polygon;
use crate::geo::shape::polygon::JoinStyle;
use crate::types::{Point, Point3D, Polygon, Rect};
use std::collections::HashSet;

pub struct ClearedArea {
    fragments: Vec<Polygon>,
    grid: SpatialGrid,
    cell_size: f64,
    // ── Batched step expansion ──
    /// Buffer of swept polygons accumulated while a batch is open.
    batch_buffer: Vec<Polygon>,
    /// True when `begin_step_batch()` has been called.
    batch_active: bool,
    /// How fragment-merging unions are performed.
    update_strategy: UpdateStrategy,
}

/// How the cleared area merges new swept polygons into stored fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateStrategy {
    /// Union ALL fragments with the new polygon(s) in one pass.
    #[default]
    Global,
    /// Only union fragments whose bbox overlaps the new polygon.
    Local,
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
    /// Optional set of polygons defining the valid tool-centre region.
    /// When set, [`step`](ClearedArea::step) checks that candidate
    /// positions stay inside this area and returns
    /// [`BoundaryHit`](StepStatus::BoundaryHit) when the tool exits it.
    pub valid_area: Option<Vec<Polygon>>,
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
            valid_area: None,
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
    ///
    /// When a candidate position falls outside the optional `valid_area`
    /// its engagement is reported as infinite so the solver naturally
    /// steers away from the boundary — the tool slides along the wall
    /// instead of trying to cross it.
    pub fn step(
        &self,
        pos: Point,
        heading: f64,
        opts: &StepperOptions,
    ) -> StepResult {
        // Helper: true when point is inside the valid tool-centre region.
        let point_is_valid = |pt: Point| -> bool {
            let Some(ref area) = opts.valid_area else {
                return true;
            };
            if area.is_empty() {
                return false;
            }
            let mut inside_outer = false;
            let mut inside_hole = false;
            for poly in area {
                if poly.len() < 3 {
                    continue;
                }
                let is_ccw = get_polygon_signed_area(poly) > 0.0;
                let inside = is_point_in_polygon(pt, poly);
                if is_ccw && inside {
                    inside_outer = true;
                } else if !is_ccw && inside {
                    inside_hole = true;
                }
            }
            inside_outer && !inside_hole
        };

        let engagement_at = |phi: f64| -> f64 {
            let dir = Point::new(phi.cos(), phi.sin());
            let candidate = pos + dir * opts.step_length;
            if !point_is_valid(candidate) {
                // Candidate outside valid area → report near-zero
                // engagement so the solver steers back inside.
                return 0.01;
            }
            let eng = self.point_engagement(candidate, opts.radius);
            eng.angle
        };

        let (best_phi, step_status, iters) =
            try_bracket(heading, opts, &engagement_at);

        // Check probes: if even the best probe has near-zero engagement,
        // the tool has no material ahead (not just at the current pos).
        let best_eng = engagement_at(best_phi);
        if best_eng < opts.target_engagement * 0.05 {
            return StepResult {
                next: pos,
                heading,
                engagement: Engagement {
                    angle: best_eng,
                    area: 0.0,
                    chord_depth: 0.0,
                },
                iters,
                status: StepStatus::LostEngagement,
            };
        }

        let mut step_len = opts.step_length;
        if step_status == StepStatus::Ok {
            let eng_at_best = best_eng;
            let cur_eng = self.point_engagement(pos, opts.radius);
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

        // Final valid-area check.
        let status = if point_is_valid(next_pos) {
            step_status
        } else {
            StepStatus::BoundaryHit
        };

        let eng = self.point_engagement(next_pos, opts.radius);

        StepResult {
            next: next_pos,
            heading: best_phi,
            engagement: eng,
            iters,
            status,
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
    pub fn new() -> Self {
        let cell_size = 10.0;
        ClearedArea {
            fragments: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            batch_buffer: Vec::new(),
            batch_active: false,
            update_strategy: UpdateStrategy::default(),
        }
    }

    pub fn from_polygons(initial: &[Polygon]) -> Self {
        let cell_size = 10.0;
        let mut ca = ClearedArea {
            fragments: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            batch_buffer: Vec::new(),
            batch_active: false,
            update_strategy: UpdateStrategy::default(),
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

    /// Replace all stored fragments with a new set (e.g., the new frontier
    /// after a wavefront advance).  This is O(m) in the new fragment count
    /// and avoids the O(n·m) accumulation of
    /// [`add_cleared_polygons`](Self::add_cleared_polygons).
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

    pub fn remaining(&self, bounds: &[Polygon]) -> Vec<Polygon> {
        if self.fragments.is_empty() {
            return bounds.to_vec();
        }
        if bounds.is_empty() {
            return vec![];
        }
        get_polygons_group_difference(bounds, &self.fragments)
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
                    self.grid.insert(idx, bb);
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

    // ── Batched step expansion ──

    /// Begin buffering single‑segment expansions.
    ///
    /// Subsequent calls to [`expand_step_batched`](Self::expand_step_batched)
    /// will be queued without a union.  Call
    /// [`commit_step_batch`](Self::commit_step_batch) to union all queued
    /// sweeps with the stored fragments in a single pass.
    ///
    /// Calling this while a batch is already active is a no‑op.
    #[prof]
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
    #[prof]
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
    #[prof]
    pub fn commit_step_batch(&mut self) {
        if !self.batch_active || self.batch_buffer.is_empty() {
            self.batch_active = false;
            return;
        }
        let buf = std::mem::take(&mut self.batch_buffer);
        match self.update_strategy {
            UpdateStrategy::Global => {
                let mut all_polys = self.fragments.clone();
                all_polys.extend(buf);
                self.fragments = get_polygons_union(&all_polys);
                self.rebuild_grid();
            }
            UpdateStrategy::Local => {
                let polys: Vec<Polygon> = if buf.len() <= 1 {
                    buf
                } else {
                    get_polygons_union(&buf)
                };
                for poly in polys {
                    self.apply_local_merge(&poly);
                }
            }
        }
        self.batch_active = false;
    }

    /// Set the fragment-merging strategy.
    pub fn set_update_strategy(&mut self, strategy: UpdateStrategy) {
        self.update_strategy = strategy;
    }

    /// Local union: merge `swept` only with fragments whose bbox overlaps it.
    fn apply_local_merge(&mut self, swept: &Polygon) {
        if swept.len() < 3 {
            return;
        }
        let mut to_merge: Vec<Polygon> = vec![swept.clone()];
        let mut removed: HashSet<usize> = HashSet::new();

        for _cascade in 0..2 {
            let bbox = get_polygon_bounds(to_merge.last().unwrap());
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
    pub fn compact_if_needed(&mut self, tol: f64) {
        self.compact_if_needed_threshold(tol, 50_000)
    }

    /// Like [`compact_if_needed`](Self::compact_if_needed) but with an
    /// explicit vertex-count threshold.
    pub fn compact_if_needed_threshold(&mut self, tol: f64, threshold: usize) {
        let total: usize = self.fragments.iter().map(|p| p.len()).sum();
        if total < threshold {
            return;
        }
        let frontier = self.frontier(tol);
        self.replace_fragments(frontier);
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
    /// with all nearby cleared fragments, and returns the uncleled
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
        let bb = Rect::new(
            c2.x - radius,
            c2.y - radius,
            c2.x + radius,
            c2.y + radius,
        );
        let nearby: Vec<Polygon> =
            self.query_window(bb).into_iter().cloned().collect();
        crate::geo::algo::engagement::cut_area(c1, c2, radius, &nearby)
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
    let n = poly.len();
    // Find the edge whose perpendicular projection best matches v.
    let mut best_edge = (0usize, 1usize);
    let mut best_d2 = f64::MAX;
    for i in 0..n {
        let j = (i + 1) % n;
        let a = poly[i];
        let b = poly[j];
        let (_, _, d2) = get_line_segment_closest_point(a, b, v.x, v.y);
        if d2 < best_d2 {
            best_d2 = d2;
            best_edge = (i, j);
        }
    }
    let (ei, ej) = best_edge;
    let edge_dir = poly[ej] - poly[ei];
    if edge_dir.length_squared() < 1e-12 {
        return 0.0;
    }
    // Right normal: for a CCW outer polygon this points outward.
    let outward = Point::new(edge_dir.y, -edge_dir.x);
    outward.y.atan2(outward.x)
}

impl Default for ClearedArea {
    fn default() -> Self {
        Self::new()
    }
}

use crate::geo::algo::engagement::Engagement;
use crate::geo::shape::polygon::get_polygon_signed_area;
use crate::geo::shape::polygon::is_point_in_polygon;
use crate::ops::area::ClearedArea;
use crate::types::{Point, Polygon};

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
pub(crate) fn try_bracket(
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
    /// Starting from `pos` with the given `heading` (radians), propose
    /// candidate positions at `step_length` distance along trial deflection
    /// angles and solve for the heading that maintains the target engagement.
    pub fn step(
        &self,
        pos: Point,
        heading: f64,
        opts: &StepperOptions,
    ) -> StepResult {
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
                return 0.01;
            }
            let eng = self.point_engagement(candidate, opts.radius);
            eng.angle
        };

        let (best_phi, step_status, iters) =
            try_bracket(heading, opts, &engagement_at);

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

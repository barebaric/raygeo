use prof_macros::prof;

use crate::dbg_log;
use crate::geo::algo::engagement::Engagement;
use crate::geo::algo::rootfind::{self, RootStatus};
use crate::ops::cut::interp::{point_in_valid_area, rotate, Interpolation};
use crate::ops::cut::ClearedArea;
use crate::types::{Point, Polygon};

/// Penalty weight applied to fresh material on the wrong side of the
/// tool (relative to `dir_sign`) when ranking candidate deflections.
///
/// When the tool breaks through a web between two cleared regions,
/// material exists on both sides and the raw `cut_area` is nearly
/// identical for left/right deflections.  Adding `DIR_BIAS_WEIGHT ×
/// wrong_side_area / step_length` to the effective error makes the
/// solver prefer the side that respects `cut_direction`.
///
/// `1.0` means a fully-wrong-side step is penalised by one full target
/// area-per-distance — strong enough to flip a tie but vanishes
/// naturally in normal one-sided cutting (where `wrong_side ≈ 0`).
const DIR_BIAS_WEIGHT: f64 = 1.0;

/// Deflection tiebreaker weight for path smoothing.
///
/// Added to `effective_err` as `DAMPING × |angle| × target_area_pd`.
/// Near the pocket boundary many candidate directions fall outside the
/// valid area and get skipped, leaving the solver with few samples and
/// causing wild heading oscillations (e.g. +30° one step, −30° the
/// next).  This linear penalty on |angle| biases the solver toward
/// smaller corrections when cut areas are similar — it acts as a
/// tiebreaker that prefers the gentler turn.
///
/// The penalty is always well below the convergence threshold
/// (`target_area_pd × 0.01`), so it never prevents legitimate
/// convergence; it only shifts the ranking when errors are close.
///
///   | deflection | penalty fraction of max_err |
///   |------------|----------------------------|
///   | 1°         |          0.9 %             |
///   | 5°         |          4.4 %             |
///   | 10°        |          8.8 %             |
///   | 20°        |         17.5 %             |
///   | 30°        |         26.2 %             |
const DEFLECTION_DAMPING: f64 = 0.005;

/// Which engagement metric the solver targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EngagementMetric {
    /// Use `Engagement.angle` (radians). Default.
    #[default]
    Angle,
    /// Use `Engagement.area` (mm²). `target_engagement` is still the
    /// equivalent angle in radians; the target area is derived from it.
    Area,
}

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
    /// Which engagement metric the solver targets.
    pub metric: EngagementMetric,
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
            metric: EngagementMetric::Angle,
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
    /// The incremental cut area (crescent) for this step.
    pub cut_area: f64,
    /// Solver iterations consumed.
    pub iters: usize,
    /// Iteration angle.
    pub iteration_angle: f64,
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
#[prof]
pub(crate) fn try_bracket(
    heading: f64,
    max_deflection: f64,
    target: f64,
    engagement_at: &dyn Fn(f64) -> f64,
) -> (f64, StepStatus, usize) {
    let (root, status, iters) =
        rootfind::bracket_grid(heading, max_deflection, |phi| {
            engagement_at(phi) - target
        });
    let step_status = match status {
        RootStatus::NoBracket | RootStatus::Converged => StepStatus::Ok,
        _ => StepStatus::NoConvergence,
    };
    (root, step_status, iters)
}

/// Perform one forward step.
///
/// Starting from `pos` with the given `heading` (radians), propose
/// candidate positions at `step_length` distance along trial deflection
/// angles and solve for the heading that maintains the target engagement.
#[prof]
pub fn step(
    cleared: &ClearedArea,
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
        point_in_valid_area(pt, area)
    };

    let (target_val, use_area): (f64, bool) = match opts.metric {
        EngagementMetric::Angle => (opts.target_engagement, false),
        EngagementMetric::Area => {
            let ta = opts.target_engagement;
            let target = opts.radius * opts.radius * 0.5 * (ta - ta.sin());
            (target, true)
        }
    };

    let engagement_at = |phi: f64| -> f64 {
        let dir = Point::new(phi.cos(), phi.sin());
        let candidate = pos + dir * opts.step_length;
        if !point_is_valid(candidate) {
            return 0.01;
        }
        let eng = cleared.point_engagement(candidate, opts.radius);
        if use_area {
            eng.area
        } else {
            eng.angle
        }
    };

    let (best_phi, step_status, iters) =
        try_bracket(heading, opts.max_deflection, target_val, &engagement_at);

    let best_val = engagement_at(best_phi);
    if best_val < target_val * 0.05 {
        return StepResult {
            next: pos,
            heading,
            engagement: cleared.point_engagement(pos, opts.radius),
            cut_area: 0.0,
            iters,
            iteration_angle: 0.0,
            status: StepStatus::LostEngagement,
        };
    }

    let mut step_len = opts.step_length;
    if step_status == StepStatus::Ok {
        let cur_eng = cleared.point_engagement(pos, opts.radius);
        let cur_val = if use_area {
            cur_eng.area
        } else {
            cur_eng.angle
        };
        let cur_err = cur_val - target_val;
        let best_err = best_val - target_val;
        if cur_err * best_err < 0.0 && cur_err.abs() > opts.engagement_tol {
            let t = cur_err / (cur_err - best_err);
            step_len *= t.clamp(0.25, 1.0);
        } else if best_err > opts.engagement_tol
            && cur_err.abs() <= opts.engagement_tol
        {
            let t = (target_val - cur_val) / (best_val - cur_val);
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

    let eng = cleared.point_engagement(next_pos, opts.radius);

    StepResult {
        next: next_pos,
        heading: best_phi,
        engagement: eng,
        cut_area: 0.0,
        iters,
        iteration_angle: 0.0,
        status,
    }
}

/// Drive the disk forward calling [`step`] until a non‑`Ok` status or
/// `max_steps` is reached.
///
/// Returns the centre path and the final status.
/// Does **not** modify the `ClearedArea` — the caller is responsible for
/// committing swept polygons after the segment.
#[prof]
pub fn run_segment(
    cleared: &ClearedArea,
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
        let result = step(cleared, pos, heading, opts);
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

/// Iterative bracketing step with cut-area engagement.
///
/// `dir_sign` is the directional bias applied when material exists on
/// both sides of the tool (a "breakthrough" between two cleared
/// regions).  Pass `+1.0` to prefer positive angles (CW), `−1.0` for
/// negative angles (CCW), or `0.0` for no bias.  On normal one-sided
/// cuts the bias has no effect (the wrong-side area is ~0).
#[allow(clippy::too_many_arguments)]
#[prof]
pub fn step_adaptive(
    cleared: &ClearedArea,
    pos: Point,
    heading: f64,
    predicted_angle: f64,
    target_area_pd: f64,
    step_length: f64,
    radius: f64,
    max_deflection: f64,
    valid_area: &[Polygon],
    angle_min: f64,
    angle_max: f64,
    dir_sign: f64,
) -> StepResult {
    let target_area = target_area_pd * step_length;
    // Hard ceiling: when the cut_area approaches the full crescent
    // (disk(c2) − disk(c1)), the tool is cutting on both sides — a
    // slot.  This corresponds to ~100° of new-material contact.  The
    // full crescent area = π·R² − lens(step, R); 95 % of it is the
    // slot-detection threshold.
    let full_crescent = std::f64::consts::PI * radius * radius
        - 2.0
            * radius
            * radius
            * ((step_length / (2.0 * radius)).clamp(-1.0, 1.0).acos())
        + (step_length * 0.5)
            * (4.0 * radius * radius - step_length * step_length)
                .max(0.0)
                .sqrt();
    let slot_ceiling = full_crescent * 0.95;
    // Max engagement: the cut_area above which the tool is taking too
    // heavy a bite.  85 % of the full crescent — above corner-wrap
    // transients (~60 %) but below the slot ceiling (95 %).
    let max_engagement = full_crescent * 0.85;
    let floor = target_area * 0.01;

    step_adaptive_inner(
        cleared,
        pos,
        heading,
        predicted_angle,
        target_area_pd,
        step_length,
        radius,
        max_deflection,
        valid_area,
        angle_min,
        angle_max,
        floor,
        max_engagement,
        slot_ceiling,
        dir_sign,
    )
}

/// Inner solver loop: evaluates candidate deflection angles and picks
/// the one whose cut_area is closest to `target_area_pd · step_length`.
///
/// Status logic:
/// * `Ok` — a candidate achieved target engagement (or close enough).
/// * `LostEngagement` — best candidate is below `floor` (under-engaged)
///   or above `slot_ceiling` (slot / overload — hard ceiling).
/// * `NoConvergence` — unused (reserved for future step-size adaptation).
#[prof]
#[allow(clippy::too_many_arguments)]
fn step_adaptive_inner(
    cleared: &ClearedArea,
    pos: Point,
    heading: f64,
    predicted_angle: f64,
    target_area_pd: f64,
    step_length: f64,
    radius: f64,
    max_deflection: f64,
    valid_area: &[Polygon],
    angle_min: f64,
    angle_max: f64,
    floor: f64,
    max_engagement: f64,
    slot_ceiling: f64,
    dir_sign: f64,
) -> StepResult {
    let base_dir = Point::new(heading.cos(), heading.sin());
    let max_err = target_area_pd * 0.01;

    let mut interp = Interpolation::new(angle_min, angle_max);
    let mut found_area = false;
    let mut best_angle = 0.0;
    let mut best_dir = base_dir;
    let mut best_pos = pos;
    let mut best_error: f64 = f64::MAX;
    let mut best_area: f64 = 0.0;
    let mut last_angle = 0.0_f64;
    let mut iters = 0;
    let mut skip_count = 0;
    let mut exit_reason = "max_iters";

    const MAX_IT: usize = 20;
    dbg_log!(
        "SA  pos=({:.3},{:.3})  heading={:.4}  pred={:.4}  \
         target_apd={:.4}  step_len={:.3}  R={:.1}  max_def={:.2}  \
         dir_sign={:+.1}",
        pos.x,
        pos.y,
        heading,
        predicted_angle,
        target_area_pd,
        step_length,
        radius,
        max_deflection,
        dir_sign,
    );
    for iter in 0..MAX_IT {
        iters = iter + 1;
        if skip_count > 3 {
            exit_reason = "skip_limit";
            break;
        }
        let (angle, is_not_interp) = match iter {
            0 => (predicted_angle, true),
            1 => (interp.min_angle(), true),
            2 => {
                if interp.joint_is_valid() {
                    (interp.interpolate(), false)
                } else {
                    (interp.max_angle(), true)
                }
            }
            _ if !found_area => {
                dbg_log!("  iter {}  LOST  (no area found)", iter);
                exit_reason = "lost_engagement";
                break;
            }
            _ => (interp.interpolate(), false),
        };

        let angle = interp.clamp_angle(angle, max_deflection);
        let dir = rotate(base_dir, angle);
        let candidate = pos + dir * step_length;
        if !point_in_valid_area(candidate, valid_area) {
            dbg_log!(
                "  iter {}  SKIP  angle={:+.4}  reason=outside_valid",
                iter,
                angle,
            );
            continue;
        }

        if interp.has_pos(candidate) {
            skip_count += 1;
            dbg_log!(
                "  iter {}  SKIP  angle={:+.4}  reason=dup_pos",
                iter,
                angle,
            );
            continue;
        }
        skip_count = 0;

        let (total, left) = cleared.cut_area_split(pos, candidate, radius);
        let right = total - left;
        let area_pd = total / step_length;
        let error = area_pd - target_area_pd;
        let is_conv = total > 0.0 && angle > 0.03;

        // Directional bias: when material is on both sides (breakthrough),
        // penalise the side that contradicts `dir_sign`.  The penalty is
        // proportional to the wrong-side area and is added to the raw
        // error for ranking — so `best_error` prefers the correct side.
        // Convergence below still checks raw error, but also rejects
        // candidates where wrong-side cutting dominates (>50 % of total),
        // preventing the solver from locking into a wrong-direction drift.
        let (effective_err, bias, wrong) = if dir_sign == 0.0 {
            (error, 0.0, 0.0)
        } else {
            // dir_sign < 0 (CCW): material should be on the right; the
            //   wrong side is `left`.
            // dir_sign > 0 (CW):  material should be on the left; the
            //   wrong side is `right`.
            let wrong = if dir_sign < 0.0 { left } else { right };
            let penalty = DIR_BIAS_WEIGHT * wrong / step_length;
            (error + penalty, penalty, wrong)
        };

        // Deflection damping: a gentle |angle| penalty that acts as a
        // tiebreaker in the `best_error` ranking.  When two candidates
        // have similar cut areas, the solver prefers the smaller
        // deflection — this smooths the path near the boundary.
        let deflection_penalty =
            DEFLECTION_DAMPING * angle.abs() * target_area_pd;
        let effective_err = effective_err + deflection_penalty;

        let iter_kind = if is_not_interp { "SMPL" } else { "INTR" };
        dbg_log!(
            "  iter {:2} {}  angle={:+.4}  apd={:.4}  err={:+.4}  \
             conv={}  |err|={:.4}  best|err|={:.4}  L={:.4}  R={:.4}  \
             bias={:+.4}  damp={:.4}",
            iter,
            iter_kind,
            angle,
            area_pd,
            error,
            is_conv as u8,
            error.abs(),
            best_error,
            left,
            right,
            bias,
            deflection_penalty,
        );

        if total > 0.0 {
            found_area = true;
        }

        interp.add(error, angle, candidate, is_not_interp, is_conv);

        last_angle = angle;
        if effective_err.abs() < best_error {
            best_error = effective_err.abs();
            best_area = total;
            best_angle = angle;
            best_dir = dir;
            best_pos = candidate;
        }

        // Convergence requires raw cut-area error within tolerance
        // AND that wrong-side cutting does not dominate.  The wrong-side
        // check prevents the solver from locking into a wrong-direction
        // drift after a breakthrough where the tool cuts primarily on
        // the wrong side.  An absolute threshold on wrong-side area is
        // used (relative to target) so that normal side cutting near
        // islands or corners is not penalised.
        let wrong_dominated = dir_sign != 0.0
            && wrong > target_area_pd * step_length * 0.5
            && wrong > right;
        if error.abs() < max_err && !wrong_dominated {
            exit_reason = "converged";
            dbg_log!(
                "  → ACCEPTED  angle={:+.4}  err={:+.4} < max_err={:.4}  \
                 wrong={:.4}  right={:.4}",
                angle,
                error,
                max_err,
                wrong,
                right,
            );
            break;
        }
    }

    // ── Status decision ──────────────────────────────────────────
    //
    // Three rules govern engagement:
    //
    // 1. **Under-engaged** (`best_area < floor`): the tool is in open
    //    space or the nearest material is more than one step away.
    //    → `LostEngagement` (let the resume machinery reposition).
    //
    // 2. **Slot / overload** (`best_area > slot_ceiling`): the
    //    cut_area approaches the full crescent — the tool is cutting
    //    on both sides (a slot).  ~100° of the perimeter is in contact
    //    with new material.  → `LostEngagement` (hard ceiling).
    //
    // 3. **Over-engaged but not a slot** (`best_area > max_engagement`
    //    and no candidate converged on target): the tool is taking too
    //    heavy a bite.  → `LostEngagement` (reposition via resume).
    //
    // Only under-engagement triggers the lookahead override; slot
    // overload is a hard stop.
    let mut status = if best_area < floor
        || best_area > slot_ceiling
        || (best_area > max_engagement && exit_reason != "converged")
    {
        StepStatus::LostEngagement
    } else {
        StepStatus::Ok
    };

    // Lookahead probes: for a spread of deflection angles relative to
    // the heading, compute the 2-step cut_area (candidate →
    // candidate+dir*step).  If the solver reversed or lost engagement
    // (but NOT from slot overload) and a forward 2-step probe finds
    // material within the valid engagement band, override to continue
    // toward it.
    let reversed = best_angle.abs() > std::f64::consts::FRAC_PI_2;
    if status == StepStatus::LostEngagement && best_area < floor || reversed {
        let lookahead_angles = [
            0.0_f64,
            max_deflection * 0.5,
            -max_deflection * 0.5,
            max_deflection,
            -max_deflection,
        ];
        let mut best_la_angle = 0.0_f64;
        let mut best_la_dir = base_dir;
        let mut best_la_pos = pos;
        let mut best_la_area: f64 = 0.0;
        // Effective score = raw area − bias penalty, so the lookahead
        // prefers directions that respect `dir_sign` when material is
        // on both sides.
        let mut best_la_score: f64 = f64::MIN;
        for la_angle in &lookahead_angles {
            let la_angle = interp.clamp_angle(*la_angle, max_deflection);
            let la_dir = rotate(base_dir, la_angle);
            let la_cand = pos + la_dir * step_length;
            if !point_in_valid_area(la_cand, valid_area) {
                continue;
            }
            let la_next = la_cand + la_dir * step_length;
            if !point_in_valid_area(la_next, valid_area) {
                continue;
            }
            let (la_area, la_left) =
                cleared.cut_area_split(la_cand, la_next, radius);
            let la_right = la_area - la_left;
            let wrong = if dir_sign < 0.0 {
                la_left
            } else if dir_sign > 0.0 {
                la_right
            } else {
                0.0
            };
            let penalty = DIR_BIAS_WEIGHT * wrong / step_length;
            let score = la_area - penalty * step_length;
            if score > best_la_score {
                best_la_score = score;
                best_la_angle = la_angle;
                best_la_dir = la_dir;
                best_la_pos = la_cand;
                // Keep `best_la_area` as the raw area for the floor /
                // slot-ceiling check below.
                best_la_area = la_area;
            }
        }
        if best_la_area > floor && best_la_area <= slot_ceiling {
            dbg_log!(
                "  LOOKAHEAD  recovered: angle={:+.4}  \
                 la_area={:.4}  → ({:.3},{:.3})  reason={}",
                best_la_angle,
                best_la_area,
                best_la_pos.x,
                best_la_pos.y,
                if status == StepStatus::LostEngagement {
                    "lost"
                } else {
                    "reversed"
                },
            );
            best_angle = best_la_angle;
            best_dir = best_la_dir;
            best_pos = best_la_pos;
            status = StepStatus::Ok;
        }
    }

    dbg_log!(
        "  RESULT  best_angle={:+.4}  last_angle={:+.4}  iters={}  \
         reason={}  status={:?}{}",
        best_angle,
        last_angle,
        iters,
        exit_reason,
        status,
        if (best_angle - last_angle).abs() > 1e-6 {
            "  ← MISMATCH (best ≠ last)"
        } else {
            ""
        },
    );

    let eng = cleared.point_engagement(best_pos, radius);
    StepResult {
        next: best_pos,
        heading: best_dir.y.atan2(best_dir.x),
        engagement: eng,
        cut_area: best_area,
        iters,
        iteration_angle: best_angle,
        status,
    }
}

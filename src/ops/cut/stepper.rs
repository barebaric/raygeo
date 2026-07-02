use prof_macros::prof;

use crate::dbg_log;
use crate::geo::algo::engagement::Engagement;
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

/// Symmetric angle bound for step search (±π/4).
///
/// Used as the default `angle_min` / `angle_max` in [`StepperOptions`] and
/// [`Interpolation`], and by the adaptive clearing main loop.
pub(crate) const STEP_ANGLE_BOUND: f64 = std::f64::consts::FRAC_PI_4;

/// Configuration options for [`step`].
///
/// Holds all parameters that stay constant across multiple step calls.
/// Only the per-step state (`cleared`, `pos`, `heading`, `predicted_angle`)
/// is passed as separate arguments.
#[derive(Debug, Clone)]
pub struct StepperOptions<'a> {
    /// Target cut-area per unit distance.
    pub target_area_pd: f64,
    /// Forward step length in mm.
    pub step_length: f64,
    /// Disk radius in mm.
    pub radius: f64,
    /// Maximum steering deflection in radians.
    pub max_deflection: f64,
    /// Valid tool-centre region polygons.
    pub valid_area: &'a [Polygon],
    /// Minimum trial deflection angle in radians (default `-π/4`).
    pub angle_min: f64,
    /// Maximum trial deflection angle in radians (default `+π/4`).
    pub angle_max: f64,
    /// Directional bias sign: `+1.0` for CW, `-1.0` for CCW, `0.0` for none.
    pub dir_sign: f64,
}

impl Default for StepperOptions<'_> {
    fn default() -> Self {
        Self {
            target_area_pd: 0.0,
            step_length: 1.0,
            radius: 5.0,
            max_deflection: std::f64::consts::FRAC_PI_6,
            valid_area: &[],
            angle_min: -STEP_ANGLE_BOUND,
            angle_max: STEP_ANGLE_BOUND,
            dir_sign: 0.0,
        }
    }
}

/// Maximum solver iterations before giving up on a single step.
pub(crate) const MAX_IT: usize = 20;

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

/// Iterative bracketing step with cut-area engagement.
///
/// `dir_sign` in [`StepperOptions`] is the directional bias applied when
/// material exists on both sides of the tool (a "breakthrough" between
/// two cleared regions).  Pass `+1.0` to prefer positive angles (CW),
/// `−1.0` for negative angles (CCW), or `0.0` for no bias.  On normal
/// one-sided cuts the bias has no effect (the wrong-side area is ~0).
#[prof]
pub fn step(
    cleared: &ClearedArea,
    pos: Point,
    heading: f64,
    predicted_angle: f64,
    opts: &StepperOptions,
) -> StepResult {
    let target_area = opts.target_area_pd * opts.step_length;
    // Hard ceiling: when the cut_area approaches the full crescent
    // (disk(c2) − disk(c1)), the tool is cutting on both sides — a
    // slot.  This corresponds to ~100° of new-material contact.  The
    // full crescent area = π·R² − lens(step, R); 95 % of it is the
    // slot-detection threshold.
    let full_crescent = std::f64::consts::PI * opts.radius * opts.radius
        - 2.0
            * opts.radius
            * opts.radius
            * ((opts.step_length / (2.0 * opts.radius))
                .clamp(-1.0, 1.0)
                .acos())
        + (opts.step_length * 0.5)
            * (4.0 * opts.radius * opts.radius
                - opts.step_length * opts.step_length)
                .max(0.0)
                .sqrt();
    let slot_ceiling = full_crescent * 0.95;
    // Max engagement: the cut_area above which the tool is taking too
    // heavy a bite.  85 % of the full crescent — above corner-wrap
    // transients (~60 %) but below the slot ceiling (95 %).
    let max_engagement = full_crescent * 0.85;
    let floor = target_area * 0.01;

    step_inner(
        cleared,
        pos,
        heading,
        predicted_angle,
        opts,
        floor,
        max_engagement,
        slot_ceiling,
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
fn step_inner(
    cleared: &ClearedArea,
    pos: Point,
    heading: f64,
    predicted_angle: f64,
    opts: &StepperOptions,
    floor: f64,
    max_engagement: f64,
    slot_ceiling: f64,
) -> StepResult {
    let base_dir = Point::new(heading.cos(), heading.sin());
    let max_err = opts.target_area_pd * 0.01;

    let mut interp = Interpolation::new(opts.angle_min, opts.angle_max);
    let mut found_area = false;
    let mut best_angle = 0.0;
    let mut best_dir = base_dir;
    let mut best_pos = pos;
    let mut best_error: f64 = f64::MAX;
    let mut best_area: f64 = 0.0;
    let mut best_left: f64 = 0.0;
    let mut last_angle = 0.0_f64;
    let mut iters = 0;
    let mut skip_count = 0;
    let mut exit_reason = "max_iters";

    dbg_log!(
        "SA  pos=({:.3},{:.3})  heading={:.4}  pred={:.4}  \
         target_apd={:.4}  step_len={:.3}  R={:.1}  max_def={:.2}  \
         dir_sign={:+.1}",
        pos.x,
        pos.y,
        heading,
        predicted_angle,
        opts.target_area_pd,
        opts.step_length,
        opts.radius,
        opts.max_deflection,
        opts.dir_sign,
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

        let angle = interp.clamp_angle(angle, opts.max_deflection);
        let dir = rotate(base_dir, angle);
        let candidate = pos + dir * opts.step_length;
        if !point_in_valid_area(candidate, opts.valid_area) {
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

        let (total, left) = cleared.cut_area_split(pos, candidate, opts.radius);
        let right = total - left;
        let area_pd = total / opts.step_length;
        let error = area_pd - opts.target_area_pd;
        let is_conv = total > 0.0 && angle > 0.03;

        // Directional bias: when material is on both sides (breakthrough),
        // penalise the side that contradicts `dir_sign`.  The penalty is
        // proportional to the wrong-side area and is added to the raw
        // error for ranking — so `best_error` prefers the correct side.
        // Convergence below still checks raw error, but also rejects
        // candidates where wrong-side cutting dominates (>50 % of total),
        // preventing the solver from locking into a wrong-direction drift.
        let (effective_err, bias, wrong) = if opts.dir_sign == 0.0 {
            (error, 0.0, 0.0)
        } else {
            // opts.dir_sign < 0 (CCW): material should be on the right; the
            //   wrong side is `left`.
            // opts.dir_sign > 0 (CW):  material should be on the left; the
            //   wrong side is `right`.
            let wrong = if opts.dir_sign < 0.0 { left } else { right };
            let penalty = DIR_BIAS_WEIGHT * wrong / opts.step_length;
            (error + penalty, penalty, wrong)
        };

        // Deflection damping: a gentle |angle| penalty that acts as a
        // tiebreaker in the `best_error` ranking.  When two candidates
        // have similar cut areas, the solver prefers the smaller
        // deflection — this smooths the path near the boundary.
        let deflection_penalty =
            DEFLECTION_DAMPING * angle.abs() * opts.target_area_pd;
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

        interp.add(error, angle, candidate);

        last_angle = angle;
        if effective_err.abs() < best_error {
            best_error = effective_err.abs();
            best_area = total;
            best_left = left;
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
        let wrong_dominated = opts.dir_sign != 0.0
            && wrong > opts.target_area_pd * opts.step_length * 0.5
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

    // ── Wrong-side rejection ────────────────────────────────────
    // Even when no single candidate converged, reject the best
    // candidate if wrong-side cutting dominates (>50 % of target
    // per-step area and more than the correct side).  The convergence
    // check inside the loop already prevents *converging* on a
    // wrong-dominated angle; this post-loop guard prevents
    // *accepting* one when every candidate was wrong-dominated.
    let best_right = best_area - best_left;
    let best_wrong = if opts.dir_sign < 0.0 {
        best_left
    } else {
        best_right
    };
    let best_wrong_dominated = opts.dir_sign != 0.0
        && best_wrong > opts.target_area_pd * opts.step_length * 0.5
        && best_wrong > best_right;

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
    // 4. **Wrong-dominated** (`best_wrong_dominated`): the best
    //    candidate has most material on the wrong side.  The solver
    //    already prevented convergence; now also block acceptance.
    //    → `LostEngagement` (reposition via resume).
    //
    // Only under-engagement triggers the lookahead override; slot
    // overload and wrong-dominated are hard stops.
    let mut status = if best_area < floor
        || best_area > slot_ceiling
        || (best_area > max_engagement && exit_reason != "converged")
        || best_wrong_dominated
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
            opts.max_deflection * 0.5,
            -opts.max_deflection * 0.5,
            opts.max_deflection,
            -opts.max_deflection,
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
            let la_angle = interp.clamp_angle(*la_angle, opts.max_deflection);
            let la_dir = rotate(base_dir, la_angle);
            let la_cand = pos + la_dir * opts.step_length;
            if !point_in_valid_area(la_cand, opts.valid_area) {
                continue;
            }
            let la_next = la_cand + la_dir * opts.step_length;
            if !point_in_valid_area(la_next, opts.valid_area) {
                continue;
            }
            let (la_area, la_left) =
                cleared.cut_area_split(la_cand, la_next, opts.radius);
            let la_right = la_area - la_left;
            let wrong = if opts.dir_sign < 0.0 {
                la_left
            } else if opts.dir_sign > 0.0 {
                la_right
            } else {
                0.0
            };
            let penalty = DIR_BIAS_WEIGHT * wrong / opts.step_length;
            let score = la_area - penalty * opts.step_length;
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

    let eng = cleared.point_engagement(best_pos, opts.radius);
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

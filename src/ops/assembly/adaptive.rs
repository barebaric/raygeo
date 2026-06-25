//! Adaptive Clearing orchestrator (forward-stepping walking path).
//!
//! Drives a [`Tool`] forward in a single continuous spiral from the seed
//! clearing to the pocket wall.  The cleared area is expanded **per step**
//! so the tool naturally spirals outward: each step's capsule blocks the
//! backward direction, and the angular engagement solver — aided by the
//! tool's heading momentum — steers into fresh material.
//!
//! The caller is responsible for pre-populating the `ClearedArea` with
//! entry polygons (e.g. via `adaptive_entry`).

use crate::geo::algo::offset::compute_inset_region;
use crate::geo::shape::polygon::get_polygon_centroid;
use crate::geo::shape::polygon::get_polygon_signed_area;
use crate::geo::shape::polygon::{get_polygon_area, is_point_in_polygon};
use crate::ops::area::ClearedArea;
use crate::ops::area::StepStatus;
use crate::ops::area::UpdateStrategy;
use crate::ops::container::Ops;
use crate::ops::state::State;
use crate::prof::prof_report;
use crate::types::{Point, Polygon};
use prof_macros::prof;

// ── Named constants ────────────────────────────────────────────────

/// Floor fraction of target cut-area-per-distance below which we treat
/// engagement as lost.
const ENGAGEMENT_FLOOR_FRAC: f64 = 0.05;
/// Bias weight applied as a fraction of `target_area_pd` when probing
/// toward the bias direction during recovery.
const BIAS_WEIGHT_FRAC: f64 = 0.15;
/// Small heading-distance penalty used as tiebreaker when no zero-
/// crossing is found (prevents direction reversal on symmetric
/// engagement landscapes).
const HEADING_TIEBREAK: f64 = 0.01;
/// Number of directions sampled during the recovery bias search.
const BIAS_PROBES: usize = 8;
/// How many multiples of `max_deflection` the bias-sampled heading may
/// be clamped away from the current heading.
const BIAS_CLAMP_FACTOR: f64 = 2.0;
/// Tool-radius fraction for the direct-jump distance in the second
/// recovery attempt.
const TRAVEL_JUMP_FRAC: f64 = 0.5;
/// Rapid feed rate used for travel moves during recovery.
const TRAVEL_RAPID_RATE: i32 = 8000;
/// Maximum total steps before giving up (safety valve).
const MAX_TOTAL_STEPS: usize = 50_000;

// ── Options ──────────────────────────────────────────────────────────

/// Options for [`adaptive_clearing`].
#[derive(Clone, Debug)]
pub struct AdaptiveClearingOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub radius: f64,
    pub advance: f64,
    pub cut_z: f64,
    pub safe_z: f64,
    pub step_length: f64,
    pub max_deflection_deg: f64,
    pub wall_margin: f64,
    pub area_tolerance: f64,
    /// Initial tool position.  When `None`, the starting position is
    /// auto-detected from the cleared-area frontier.
    pub start_pos: Option<Point>,
    /// Initial tool heading in radians.  When `None`, the heading is
    /// auto-detected as the CCW tangent at the starting position.
    pub start_heading: Option<f64>,
    /// How many steps to accumulate before committing cleared-area
    /// expansions.  Larger values reduce per‑step overhead at the cost
    /// of slightly stale engagement queries.  Leave at 1 (default) for
    /// best path quality; increase to 5+ for faster roughing passes.
    pub expansion_batch_size: usize,
}

impl Default for AdaptiveClearingOptions {
    fn default() -> Self {
        Self {
            pocket_boundary: Vec::new(),
            islands: Vec::new(),
            radius: 3.0,
            advance: 1.5,
            cut_z: -5.0,
            safe_z: 2.0,
            step_length: 0.6,
            max_deflection_deg: 30.0,
            wall_margin: 0.0,
            area_tolerance: 1.0,
            start_pos: None,
            start_heading: None,
            expansion_batch_size: 1,
        }
    }
}

// ── Tool ─────────────────────────────────────────────────────────────

/// A cutting tool with persistent position and heading.
///
/// The heading carries directional momentum: on symmetric engagement
/// landscapes (e.g. circular boundaries) the solver prefers the
/// direction closest to the current heading, preventing reversal.
#[derive(Clone, Copy, Debug)]
pub struct Tool {
    /// Tool centre position.
    pub pos: Point,
    /// Current heading angle (radians).
    pub heading: f64,
    /// Tool radius.
    pub radius: f64,
}

impl Tool {
    /// Perform one forward step using incremental cut-area engagement.
    ///
    /// Probes candidate angles around `self.heading`, evaluates
    /// [`cut_area`](ClearedArea::cut_area) at each, and picks the angle
    /// whose area-per-distance is closest to `target_area_pd`.
    ///
    /// When `bias_angle` is `Some(...)`, probes closer to that direction
    /// get a bonus of `bias_weight * cos(probe_angle - bias_angle)`,
    /// making the solver prefer that direction when engagement is
    /// ambiguous.
    ///
    /// When multiple angles yield similar area (symmetric landscape),
    /// the one closest to `self.heading` wins — the velocity
    /// tie-breaker that prevents direction reversal.
    #[allow(clippy::too_many_arguments)]
    #[prof]
    pub fn step(
        &mut self,
        cleared: &ClearedArea,
        target_area_pd: f64,
        max_deflection: f64,
        step_length: f64,
        valid_area: &[Polygon],
        bias_angle: Option<f64>,
        bias_weight: f64,
    ) -> StepStatus {
        let ratios = [-1.0, -0.6, -0.2, 0.0, 0.2, 0.6, 1.0];

        // ── Evaluate cut area at each probe angle ──
        let mut samples: [(f64, f64); 7] = [(0.0, 0.0); 7]; // (angle, error)
        for (i, &r) in ratios.iter().enumerate() {
            let phi = self.heading + max_deflection * r;
            let dir = Point::new(phi.cos(), phi.sin());
            let candidate = self.pos + dir * step_length;

            if !point_in_valid_area(candidate, valid_area) {
                samples[i] = (phi, -target_area_pd); // error: no area
                continue;
            }

            let area = cleared.cut_area(self.pos, candidate, self.radius);
            let area_pd = area / step_length;
            let mut err = area_pd - target_area_pd;
            if let Some(ba) = bias_angle {
                err += bias_weight * (phi - ba).cos();
            }
            samples[i] = (phi, err);
        }

        // ── Pick best angle ──
        // Among zero-crossings, prefer the one closest to heading.
        let mut best_phi = self.heading;
        let mut found_crossing = false;

        for i in 0..samples.len() - 1 {
            let (a, fa) = samples[i];
            let (b, fb) = samples[i + 1];
            if fa.is_finite() && fb.is_finite() && fa.signum() != fb.signum() {
                let t = -fa / (fb - fa);
                let root = a + t * (b - a);
                if !found_crossing {
                    best_phi = root;
                    found_crossing = true;
                } else {
                    let prev_diff =
                        angle_normalize(best_phi - self.heading).abs();
                    let new_diff = angle_normalize(root - self.heading).abs();
                    if new_diff < prev_diff {
                        best_phi = root;
                    }
                }
            }
        }

        if !found_crossing {
            // No crossing: pick sample with smallest weighted error.
            best_phi = samples
                .iter()
                .min_by(|&(a_ang, a_err), &(b_ang, b_err)| {
                    let a_w = a_err.abs()
                        + angle_normalize(a_ang - self.heading).abs()
                            * HEADING_TIEBREAK;
                    let b_w = b_err.abs()
                        + angle_normalize(b_ang - self.heading).abs()
                            * HEADING_TIEBREAK;
                    a_w.partial_cmp(&b_w).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|s| s.0)
                .unwrap_or(self.heading);
        }

        // ── Validate and commit ──
        let dir = Point::new(best_phi.cos(), best_phi.sin());
        let next_pos = self.pos + dir * step_length;

        if !point_in_valid_area(next_pos, valid_area) {
            return StepStatus::BoundaryHit;
        }

        let area = cleared.cut_area(self.pos, next_pos, self.radius);
        if area < step_length * target_area_pd * ENGAGEMENT_FLOOR_FRAC {
            return StepStatus::LostEngagement;
        }

        self.pos = next_pos;
        self.heading = best_phi;
        StepStatus::Ok
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn angle_normalize(a: f64) -> f64 {
    let mut a = a % (2.0 * std::f64::consts::PI);
    if a >= std::f64::consts::PI {
        a -= 2.0 * std::f64::consts::PI;
    }
    if a < -std::f64::consts::PI {
        a += 2.0 * std::f64::consts::PI;
    }
    a
}

/// Check whether a point lies inside the valid tool-centre region.
///
/// Uses polygon winding to distinguish outer boundaries (CCW) from holes
/// (CW): a point must be inside at least one CCW polygon and outside all
/// CW polygons.
fn point_in_valid_area(pt: Point, area: &[Polygon]) -> bool {
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
}

/// Target cut-area per unit distance for the engagement solver.
///
/// Derived from the crescent-area formula for a step of `advance` and
/// tool `radius`.
fn target_area_per_distance(radius: f64, advance: f64) -> f64 {
    let d_ref = radius * 0.5;
    let overlap = 2.0 * radius * radius * (d_ref / (2.0 * radius)).acos()
        - (d_ref / 2.0) * (4.0 * radius * radius - d_ref * d_ref).sqrt();
    let reference_cut_area = std::f64::consts::PI * radius * radius - overlap;
    let step_over_factor = advance / (2.0 * radius);
    2.0 * step_over_factor * reference_cut_area / radius
}

/// The result of a local recovery attempt.
struct Recovery {
    pos: Point,
    heading: f64,
    /// When `true` the caller must retract to safe_z, travel to `pos`,
    /// and plunge before resuming stepping.
    requires_travel: bool,
}

/// Sample 8 directions around `tool.pos` and try to re-engage uncut
/// material.
///
/// Returns `None` when every direction is either outside the valid area
/// or has zero cut area.
fn attempt_local_recovery(
    cleared: &ClearedArea,
    tool: &Tool,
    target_area_pd: f64,
    max_def: f64,
    step_length: f64,
    valid_tool_area: &[Polygon],
) -> Option<Recovery> {
    // Sample BIAS_PROBES directions and pick the one with most cut_area.
    let mut best_area = 0.0f64;
    let mut bias_angle = 0.0f64;
    for si in 0..BIAS_PROBES {
        let a = std::f64::consts::TAU * si as f64 / BIAS_PROBES as f64;
        let d = Point::new(a.cos(), a.sin());
        let cand = tool.pos + d * step_length;
        if !point_in_valid_area(cand, valid_tool_area) {
            continue;
        }
        let area = cleared.cut_area(tool.pos, cand, tool.radius);
        if area > best_area {
            best_area = area;
            bias_angle = a;
        }
    }

    if best_area <= 0.0 {
        return None;
    }

    // Attempt 1: step with bias toward the best direction.
    let diff = angle_normalize(bias_angle - tool.heading);
    let clamped =
        diff.clamp(-max_def * BIAS_CLAMP_FACTOR, max_def * BIAS_CLAMP_FACTOR);
    let biased_heading = angle_normalize(tool.heading + clamped);
    let bias_weight = target_area_pd * BIAS_WEIGHT_FRAC;
    let mut probe = Tool {
        pos: tool.pos,
        heading: biased_heading,
        radius: tool.radius,
    };
    let s = probe.step(
        cleared,
        target_area_pd,
        max_def,
        step_length,
        valid_tool_area,
        Some(bias_angle),
        bias_weight,
    );
    if s == StepStatus::Ok {
        return Some(Recovery {
            pos: probe.pos,
            heading: probe.heading,
            requires_travel: false,
        });
    }

    // Attempt 2: jump a short distance in the bias direction, then step.
    let jump_dir = Point::new(bias_angle.cos(), bias_angle.sin());
    let cand = tool.pos + jump_dir * (tool.radius * TRAVEL_JUMP_FRAC);
    if !point_in_valid_area(cand, valid_tool_area) {
        return None;
    }
    let mut land = Tool {
        pos: cand,
        heading: bias_angle,
        radius: tool.radius,
    };
    if land.step(
        cleared,
        target_area_pd,
        max_def,
        step_length,
        valid_tool_area,
        None,
        0.0,
    ) == StepStatus::Ok
    {
        return Some(Recovery {
            pos: land.pos,
            heading: land.heading,
            requires_travel: true,
        });
    }

    None
}

// ── Main entry point ─────────────────────────────────────────────────

#[prof]
pub fn adaptive_clearing(
    cleared: &mut ClearedArea,
    opts: &AdaptiveClearingOptions,
    cut_state: &State,
) -> Ops {
    // ── 1. Pre-process ────────────────────────────────────────────
    let (valid_tool_area, valid_total) =
        compute_inset_region(&opts.pocket_boundary, opts.radius, &opts.islands);
    if valid_tool_area.is_empty() || valid_total <= opts.area_tolerance {
        return Ops::new();
    }
    if cleared.is_empty() {
        return Ops::new();
    }

    let max_def = opts.max_deflection_deg.to_radians();
    let target_area_pd = target_area_per_distance(opts.radius, opts.advance);

    // ── 2. Initialise the tool ───────────────────────────────────
    let centre = cleared
        .fragments()
        .iter()
        .max_by(|a, b| {
            let aa = get_polygon_area(a);
            let ab = get_polygon_area(b);
            ab.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(get_polygon_centroid)
        .unwrap_or(Point::ZERO);

    // Use caller-provided position/heading when available (e.g. the
    // tool is already in motion after an entry strategy).  Otherwise
    // auto-detect from the cleared-area frontier.
    let frontier = cleared.frontier(0.5);
    let (default_pos, default_heading) = initial_pose(&frontier, centre);
    let start_pos = opts.start_pos.unwrap_or(default_pos);
    let start_heading = opts.start_heading.unwrap_or(default_heading);

    let mut tool = Tool {
        pos: start_pos,
        heading: start_heading,
        radius: opts.radius,
    };

    // ── 3. Continuous spiral: step → expand → repeat ─────────────
    cleared.set_update_strategy(UpdateStrategy::Local);

    let mut ops = Ops::new();
    ops.apply_state(cut_state);
    ops.move_to(tool.pos.x, tool.pos.y, opts.cut_z, None);

    let mut prev_pos = tool.pos;
    let mut steps_since_batch: usize = 0;

    for _ in 0..MAX_TOTAL_STEPS {
        // Convergence.
        if cleared.total_area() >= valid_total - opts.area_tolerance {
            break;
        }

        let status = tool.step(
            cleared,
            target_area_pd,
            max_def,
            opts.step_length,
            &valid_tool_area,
            None,
            0.0,
        );

        if status != StepStatus::Ok {
            if steps_since_batch > 0 {
                cleared.commit_step_batch();
                steps_since_batch = 0;
            }

            let recovery = attempt_local_recovery(
                cleared,
                &tool,
                target_area_pd,
                max_def,
                opts.step_length,
                &valid_tool_area,
            );

            if let Some(r) = recovery {
                if r.requires_travel {
                    ops.apply_state(&State {
                        rapid_rate: Some(TRAVEL_RAPID_RATE),
                        ..Default::default()
                    });
                    ops.move_to(tool.pos.x, tool.pos.y, opts.safe_z, None);
                    ops.move_to(r.pos.x, r.pos.y, opts.safe_z, None);
                    ops.apply_state(cut_state);
                    ops.move_to(r.pos.x, r.pos.y, opts.cut_z, None);
                }
                tool.pos = r.pos;
                tool.heading = r.heading;
                prev_pos = tool.pos;
                continue;
            }
            break;
        }

        // Emit cutting move.
        ops.line_to(tool.pos.x, tool.pos.y, opts.cut_z, None);

        // Expand cleared area.
        if steps_since_batch == 0 {
            cleared.begin_step_batch();
        }
        cleared.expand_step_batched(prev_pos, tool.pos, opts.radius);
        steps_since_batch += 1;

        if steps_since_batch >= opts.expansion_batch_size {
            cleared.commit_step_batch();
            steps_since_batch = 0;
        }

        prev_pos = tool.pos;
    }

    // Flush any remaining batch.
    if steps_since_batch > 0 {
        cleared.commit_step_batch();
    }

    ops
}

/// Wrapper around [`adaptive_clearing`] that prints a profiling report
/// to stderr when the `RAYGEO_PROFILE` environment variable is set.
pub fn adaptive_clearing_with_profile(
    cleared: &mut ClearedArea,
    opts: &AdaptiveClearingOptions,
    cut_state: &State,
    travel_state: &State,
) -> Ops {
    let result = adaptive_clearing(cleared, opts, cut_state);
    if std::env::var("RAYGEO_PROFILE").is_ok() {
        prof_report();
    }
    // travel_state is accepted for API compatibility but unused.
    let _ = travel_state;
    result
}

// ── Initial pose ─────────────────────────────────────────────────────

#[prof]
fn initial_pose(frontier: &[Polygon], centre: Point) -> (Point, f64) {
    let mut best_poly: Option<&Polygon> = None;
    let mut best_area = 0.0f64;
    for poly in frontier {
        if poly.len() < 3 {
            continue;
        }
        let area = get_polygon_area(poly);
        if area > best_area {
            best_area = area;
            best_poly = Some(poly);
        }
    }

    let poly = match best_poly {
        Some(p) => p,
        None => return (centre, 0.0),
    };

    let pos = poly[0];
    let radial = pos - centre;
    let radial_angle = radial.y.atan2(radial.x);
    let tangent_angle =
        angle_normalize(radial_angle + std::f64::consts::FRAC_PI_2);

    (pos, tangent_angle)
}

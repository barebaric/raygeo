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
use crate::geo::shape::polygon::is_point_in_polygon;
use crate::geo::shape::polygon::{get_polygon_area, get_polygon_centroid};
use crate::ops::area::target_engagement_from_advance;
use crate::ops::area::ClearedArea;
use crate::ops::area::StepStatus;
use crate::ops::area::UpdateStrategy;
use crate::ops::container::Ops;
use crate::ops::state::State;
use crate::prof::prof_report;
use crate::types::{Point, Polygon};
use prof_macros::prof;

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
    pub travel_smoothing: i32,
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
            travel_smoothing: 50,
            wall_margin: 0.0,
            area_tolerance: 1.0,
            start_pos: None,
            start_heading: None,
            expansion_batch_size: 1,
        }
    }
}

/// Maximum total steps before giving up (safety valve).
const MAX_TOTAL_STEPS: usize = 50_000;

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
    /// When multiple angles yield similar area (symmetric landscape),
    /// the one closest to `self.heading` wins — the velocity
    /// tie-breaker that prevents direction reversal.
    #[prof]
    pub fn step(
        &mut self,
        cleared: &ClearedArea,
        target_area_pd: f64,
        max_deflection: f64,
        step_length: f64,
        valid_area: &[Polygon],
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
            samples[i] = (phi, area_pd - target_area_pd);
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
                        + angle_normalize(a_ang - self.heading).abs() * 0.01;
                    let b_w = b_err.abs()
                        + angle_normalize(b_ang - self.heading).abs() * 0.01;
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
        if area < step_length * target_area_pd * 0.05 {
            return StepStatus::LostEngagement;
        }

        self.pos = next_pos;
        self.heading = best_phi;
        StepStatus::Ok
    }

    /// Like [`step`](Self::step) but adds a directional bias to the
    /// probe evaluation.  Probes closer to `bias_angle` get a bonus
    /// of `bias_weight * cos(probe_angle - bias_angle)`, making the
    /// solver prefer that direction when engagement is ambiguous.
    #[allow(clippy::too_many_arguments)]
    #[prof]
    pub fn step_with_bias(
        &mut self,
        cleared: &ClearedArea,
        target_area_pd: f64,
        max_deflection: f64,
        step_length: f64,
        valid_area: &[Polygon],
        bias_angle: f64,
        bias_weight: f64,
    ) -> StepStatus {
        let ratios = [-1.0, -0.6, -0.2, 0.0, 0.2, 0.6, 1.0];

        // ── Evaluate cut area at each probe angle ──
        let mut samples: [(f64, f64); 7] = [(0.0, 0.0); 7];
        for (i, &r) in ratios.iter().enumerate() {
            let phi = self.heading + max_deflection * r;
            let dir = Point::new(phi.cos(), phi.sin());
            let candidate = self.pos + dir * step_length;

            if !point_in_valid_area(candidate, valid_area) {
                samples[i] = (phi, -target_area_pd);
                continue;
            }

            let area = cleared.cut_area(self.pos, candidate, self.radius);
            let area_pd = area / step_length;
            let align = (phi - bias_angle).cos();
            samples[i] = (phi, area_pd - target_area_pd + bias_weight * align);
        }

        // ── Pick best angle ──
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
            best_phi = samples
                .iter()
                .min_by(|&(a_ang, a_err), &(b_ang, b_err)| {
                    let a_w = a_err.abs()
                        + angle_normalize(a_ang - self.heading).abs() * 0.01;
                    let b_w = b_err.abs()
                        + angle_normalize(b_ang - self.heading).abs() * 0.01;
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
        if area < step_length * target_area_pd * 0.05 {
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

fn point_in_valid_area(pt: Point, area: &[Polygon]) -> bool {
    let mut inside_outer = false;
    for poly in area {
        if poly.len() < 3 {
            continue;
        }
        if is_point_in_polygon(pt, poly) {
            inside_outer = !inside_outer;
        }
    }
    inside_outer
}

// ── Main entry point ─────────────────────────────────────────────────

#[prof]
pub fn adaptive_clearing(
    cleared: &mut ClearedArea,
    opts: &AdaptiveClearingOptions,
    cut_state: &State,
    _travel_state: &State,
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

    let _target_eng = target_engagement_from_advance(opts.advance, opts.radius);
    let max_def = opts.max_deflection_deg.to_radians();

    // Target cut-area per unit distance (crescent formula).
    // referenceCutArea = area of disk minus disk-offset-by-R/2 (crescent).
    let r = opts.radius;
    let d_ref = r * 0.5;
    let overlap = 2.0 * r * r * (d_ref / (2.0 * r)).acos()
        - (d_ref / 2.0) * (4.0 * r * r - d_ref * d_ref).sqrt();
    let reference_cut_area = std::f64::consts::PI * r * r - overlap;
    let step_over_factor = opts.advance / (2.0 * opts.radius);
    let target_area_pd =
        2.0 * step_over_factor * reference_cut_area / opts.radius;

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
        );

        if status != StepStatus::Ok {
            if steps_since_batch > 0 {
                cleared.commit_step_batch();
                steps_since_batch = 0;
            }

            // Try stepping with an outside bias — probe directions
            // toward uncut material get a preference bonus.
            // Compute the bias direction by sampling 8 directions
            // and picking the one with the most cut_area.
            let mut best_area = 0.0f64;
            let mut bias_angle = 0.0f64;
            for si in 0..8 {
                let a = std::f64::consts::TAU * si as f64 / 8.0;
                let d = Point::new(a.cos(), a.sin());
                let cand = tool.pos + d * opts.step_length;
                if !point_in_valid_area(cand, &valid_tool_area) {
                    continue;
                }
                let area = cleared.cut_area(tool.pos, cand, opts.radius);
                if area > best_area {
                    best_area = area;
                    bias_angle = a;
                }
            }

            if best_area > 0.0 {
                // Shift heading toward the bias direction so the probes
                // actually cover that angle range.
                let diff = angle_normalize(bias_angle - tool.heading);
                let clamped = diff.clamp(-max_def * 2.0, max_def * 2.0);
                let biased_heading = angle_normalize(tool.heading + clamped);
                let bias_weight = target_area_pd * 0.15;
                let mut probe = Tool {
                    pos: tool.pos,
                    heading: biased_heading,
                    radius: opts.radius,
                };
                let s = probe.step_with_bias(
                    cleared,
                    target_area_pd,
                    max_def,
                    opts.step_length,
                    &valid_tool_area,
                    bias_angle,
                    bias_weight,
                );
                if s == StepStatus::Ok {
                    tool.pos = probe.pos;
                    tool.heading = probe.heading;
                    prev_pos = tool.pos;
                    continue;
                }

                // Bias still failed — try direct travel to the outside-
                // biased position and resume stepping from there.
                let cand = tool.pos
                    + Point::new(bias_angle.cos(), bias_angle.sin())
                        * (opts.radius * 0.5);
                if point_in_valid_area(cand, &valid_tool_area) {
                    let mut land = Tool {
                        pos: cand,
                        heading: bias_angle,
                        radius: opts.radius,
                    };
                    if land.step(
                        cleared,
                        target_area_pd,
                        max_def,
                        opts.step_length,
                        &valid_tool_area,
                    ) == StepStatus::Ok
                    {
                        ops.apply_state(&State {
                            rapid_rate: Some(8000),
                            ..Default::default()
                        });
                        ops.move_to(tool.pos.x, tool.pos.y, opts.safe_z, None);
                        ops.move_to(cand.x, cand.y, opts.safe_z, None);
                        ops.apply_state(cut_state);
                        ops.move_to(cand.x, cand.y, opts.cut_z, None);
                        tool.pos = land.pos;
                        tool.heading = land.heading;
                        prev_pos = tool.pos;
                        continue;
                    }
                }
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
    let result = adaptive_clearing(cleared, opts, cut_state, travel_state);
    if std::env::var("RAYGEO_PROFILE").is_ok() {
        prof_report();
    }
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

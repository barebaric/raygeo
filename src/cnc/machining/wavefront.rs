//! Wavefront clearing workplan builder.
//!
//! Produces a minimal two-step workplan that seeds the pocket centre with
//! a flat spiral and then expands outward with [`adaptive_wavefronts`].
//! This is the decoupled replacement for the old `adaptive_entry` +
//! `adaptive_wavefronts` hand-wiring: the param derivation that used to
//! live inside `generate_helix_spiral` / `adaptive_entry` is centralised
//! here, and no helical plunge is emitted (the flat spiral's swept disk
//! already contains the area the helix used to cover, so the wavefront
//! seed is identical).
//!
//! Combining the steps into a single toolpath is the job of the
//! workplan executor ([`crate::cnc::machining::plan::execute_workplan`]),
//! not this builder.
//!
//! [`adaptive_wavefronts`]: crate::ops::assembly::wavefront::adaptive_wavefronts

use prof_macros::prof;

use crate::cnc::machining::plan::WorkplanStep;
use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::shape::polygon::get_polygon_centroid;
use crate::types::Polygon;

/// Options for [`build_wavefront_workplan`].
#[derive(Clone, Debug)]
pub struct WavefrontWorkplanOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub tool_radius: f64,
    pub step_over: f64,
    pub target_z: f64,
    pub safe_margin: f64,
    pub angular_step: f64,
    pub area_tolerance: f64,
    pub precision: f64,
}

/// Build a spiral-seed + wavefront-expansion workplan.
///
/// Finds the largest inscribed circle, derives the spiral radii exactly
/// as the legacy `generate_helix_spiral` did, and emits a
/// [`WorkplanStep::FlatSpiral`] (the seed) followed by a
/// [`WorkplanStep::Wavefront`] (the expansion). No helical plunge is
/// produced: the spiral's swept disk already reaches `spiral_max_r`, so
/// the wavefront seed — and therefore the resulting toolpath — is
/// unchanged.
#[prof]
pub fn build_wavefront_workplan(
    opts: &WavefrontWorkplanOptions,
) -> RaygeoResult<Vec<WorkplanStep>> {
    let (entry_pt, r_max) =
        find_largest_circle(&opts.pocket_boundary, &opts.islands, 0.1)
            .unwrap_or_else(|| {
                (get_polygon_centroid(&opts.pocket_boundary), 0.0)
            });

    let helix_r = (opts.tool_radius * 0.8).min(r_max * 0.5);
    // Same bound as the legacy `generate_helix_spiral`: the spiral always
    // reaches at least `helix_r + 0.01`, so `radial_dist` is always > 0
    // and a seed disk is always produced.
    let spiral_max_r =
        (r_max - opts.tool_radius - opts.safe_margin).max(helix_r + 0.01);
    let radial_dist = spiral_max_r - helix_r;

    let mut steps: Vec<WorkplanStep> = Vec::new();

    if radial_dist > 0.0 && opts.step_over > 0.0 {
        steps.push(WorkplanStep::FlatSpiral {
            center: entry_pt,
            z: opts.target_z,
            start_radius: helix_r,
            end_radius: spiral_max_r,
            revolutions: radial_dist / opts.step_over,
            direction: HelixDirection::Cw,
            angular_step: opts.angular_step,
            start_angle: 0.0,
        });
    }

    steps.push(WorkplanStep::Wavefront {
        pocket_boundary: opts.pocket_boundary.clone(),
        islands: opts.islands.clone(),
        tool_radius: opts.tool_radius,
        step_over: opts.step_over,
        z: opts.target_z,
        area_tolerance: opts.area_tolerance,
        precision: opts.precision,
    });

    Ok(steps)
}

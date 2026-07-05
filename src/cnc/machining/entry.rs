//! CNC entry strategy orchestration.
//!
//! Determines the best entry method for a pocket and dispatches to the
//! appropriate ops-layer generator (helix, spiral, ramp, or toroid).

use prof_macros::prof;

use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::shape::line::longest_line_through_point;
use crate::geo::shape::polygon::{get_polygon_bounds, get_polygon_centroid};
use crate::ops::assembly::helix::{self, HelixOptions};
use crate::ops::assembly::ramp::{self, RampOptions};
use crate::ops::assembly::result::{chain, AssemblyResult};
use crate::ops::assembly::spiral::{self, SpiralOptions};
use crate::ops::assembly::toroid::{self, ToroidOptions};
use crate::ops::state::State;
use crate::types::Polygon;

/// Options for [`adaptive_entry`].
#[derive(Clone, Debug)]
pub struct AdaptiveEntryOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub tool_radius: f64,
    pub step_over: f64,
    pub safe_z: f64,
    pub target_z: f64,
    pub plunge_pitch: f64,
    pub safe_margin: f64,
    pub angular_step: f64,
}

/// Entry method classification for a pocket.
pub enum EntryMethod {
    /// Wide pocket — helical plunge + flat spiral.
    HelixSpiral,
    /// Slot wide enough for trochoidal loops.
    Toroid,
    /// Tight slot — zigzag ramp.
    Ramp,
    /// Degenerate pocket, nothing to cut.
    None,
}

/// Classify a pocket by its largest inscribed circle radius.
pub fn detect_entry_method(
    r_max: f64,
    tool_radius: f64,
    _safe_margin: f64,
) -> EntryMethod {
    if r_max >= tool_radius * 1.5 {
        EntryMethod::HelixSpiral
    } else {
        EntryMethod::Ramp
    }
}

/// Fast central clearing entry.
///
/// Given a pocket boundary (with optional islands), finds the optimal
/// entry pole and dispatches to the appropriate generator:
///
/// - **Helix → Spiral** (wide area): helical plunge to depth followed by
///   a flat Archimedean spiral with smoothing circular pass.
/// - **ZigZag Ramp** (tight slot): a trochoidal ramp along the longest
///   axis of the slot.
#[prof]
pub fn adaptive_entry(
    opts: &AdaptiveEntryOptions,
    cut_state: &State,
) -> AssemblyResult {
    let (entry_pt, r_max) =
        find_largest_circle(&opts.pocket_boundary, &opts.islands, 0.1)
            .unwrap_or_else(|| {
                let c = get_polygon_centroid(&opts.pocket_boundary);
                (c, 0.0)
            });

    match detect_entry_method(r_max, opts.tool_radius, opts.safe_margin) {
        EntryMethod::HelixSpiral => {
            generate_helix_spiral(entry_pt, r_max, opts, cut_state)
        }
        EntryMethod::Toroid => {
            let bbox = get_polygon_bounds(&opts.pocket_boundary);
            let (start, end) = longest_line_through_point(entry_pt, bbox);
            toroid::generate_toroid(
                &ToroidOptions {
                    carrier: vec![start, end],
                    tool_radius: opts.tool_radius,
                    step_distance: opts.step_over,
                    z: opts.target_z,
                    direction: HelixDirection::Cw,
                    angular_step: opts.angular_step,
                },
                cut_state,
            )
        }
        EntryMethod::Ramp => {
            let bbox = get_polygon_bounds(&opts.pocket_boundary);
            let (start, end) = longest_line_through_point(entry_pt, bbox);
            ramp::generate_ramp(
                &RampOptions {
                    start,
                    end,
                    z_start: opts.safe_z,
                    z_end: opts.target_z,
                    max_ramp_angle_deg: 45.0,
                    style: crate::geo::algo::ramp::RampStyle::ZigZag,
                    lateral_amplitude: opts.tool_radius * 0.8,
                },
                cut_state,
            )
        }
        EntryMethod::None => AssemblyResult {
            ops: crate::ops::container::Ops::new(),
            cleared_polygons: vec![],
            start: crate::ops::cut::ToolPose {
                pos: entry_pt,
                heading: 0.0,
            },
            end: crate::ops::cut::ToolPose {
                pos: entry_pt,
                heading: 0.0,
            },
        },
    }
}

/// Build a helix→spiral entry sequence.
///
/// Chains a helical plunge with a flat Archimedean spiral + smoothing
/// circular pass.  Useful on its own when you already know the entry
/// point and max radius (e.g. from `find_largest_circle`).
#[prof]
pub fn generate_helix_spiral(
    entry_pt: crate::types::Point,
    r_max: f64,
    opts: &AdaptiveEntryOptions,
    cut_state: &State,
) -> AssemblyResult {
    let helix_r = (opts.tool_radius * 0.8).min(r_max * 0.5);

    let helix_result = if opts.target_z < opts.safe_z {
        helix::generate_helix(
            &HelixOptions {
                center: entry_pt,
                start_radius: helix_r,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                pitch: opts.plunge_pitch,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            },
            cut_state,
        )
    } else {
        AssemblyResult {
            ops: crate::ops::container::Ops::new(),
            cleared_polygons: vec![],
            start: crate::ops::cut::ToolPose {
                pos: entry_pt,
                heading: 0.0,
            },
            end: crate::ops::cut::ToolPose {
                pos: entry_pt,
                heading: 0.0,
            },
        }
    };

    let spiral_max_r =
        (r_max - opts.tool_radius - opts.safe_margin).max(helix_r + 0.01);
    let radial_dist = spiral_max_r - helix_r;

    if radial_dist > 0.0 && opts.step_over > 0.0 {
        let n_revs = radial_dist / opts.step_over;
        let start_angle = (helix_result.end.pos.y - entry_pt.y)
            .atan2(helix_result.end.pos.x - entry_pt.x);

        let spiral_result = spiral::generate_spiral(
            &SpiralOptions {
                center: entry_pt,
                z: opts.target_z,
                start_radius: helix_r,
                end_radius: spiral_max_r,
                revolutions: n_revs,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
                start_angle,
            },
            cut_state,
        );

        chain(helix_result, spiral_result)
    } else {
        helix_result
    }
}

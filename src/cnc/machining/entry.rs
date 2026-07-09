//! CNC entry strategy orchestration.
//!
//! Two APIs:
//!
//! - **Legacy**: [`adaptive_entry`] + [`detect_entry_method`] +
//!   [`generate_helix_spiral`] — executes the entry directly and returns
//!   an [`AssemblyResult`].  Used by downstream examples (wavefront,
//!   showcase, medial axis) and CLI scenarios.
//! - **Workplan**: [`build_entry_workplan`] — uses feature detection to
//!   produce a `Vec<WorkplanStep>` without executing.

use prof_macros::prof;

use crate::cnc::machining::plan::WorkplanStep;
use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::ramp::RampStyle;
use crate::geo::shape::line::longest_line_through_point;
use crate::geo::shape::polygon::{get_polygon_bounds, get_polygon_centroid};
use crate::ops::assembly::helix::{self, HelixOptions};
use crate::ops::assembly::ramp::{self, RampOptions};
use crate::ops::assembly::result::{chain, AssemblyResult};
use crate::ops::assembly::spiral::{self, SpiralOptions};
use crate::ops::assembly::toroid::{self, ToroidOptions};
use crate::ops::feature::ramp::find_ramp_carrier;
use crate::ops::feature::region::find_regions;
use crate::ops::state::State;
use crate::types::{Point3D, Polygon};

// ═══════════════════════════════════════════════════════════════════════
// Legacy API: adaptive_entry, detect_entry_method, generate_helix_spiral
// ═══════════════════════════════════════════════════════════════════════

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
) -> RaygeoResult<AssemblyResult> {
    let (entry_pt, r_max) =
        find_largest_circle(&opts.pocket_boundary, &opts.islands, 0.1)
            .unwrap_or_else(|| {
                let c = get_polygon_centroid(&opts.pocket_boundary);
                (c, 0.0)
            });

    match detect_entry_method(r_max, opts.tool_radius, opts.safe_margin) {
        EntryMethod::HelixSpiral => {
            Ok(generate_helix_spiral(entry_pt, r_max, opts, cut_state)?)
        }
        EntryMethod::Toroid => {
            let bbox = get_polygon_bounds(&opts.pocket_boundary);
            let (start, end) = longest_line_through_point(entry_pt, bbox);
            Ok(toroid::generate_toroid(
                &ToroidOptions {
                    carrier: vec![start, end],
                    tool_radius: opts.tool_radius,
                    step_over: opts.step_over,
                    target_z: opts.target_z,
                    direction: HelixDirection::Cw,
                    angular_step: opts.angular_step,
                },
                cut_state,
            )?)
        }
        EntryMethod::Ramp => {
            let bbox = get_polygon_bounds(&opts.pocket_boundary);
            let (start, end) = longest_line_through_point(entry_pt, bbox);
            Ok(ramp::generate_ramp(
                &RampOptions {
                    start,
                    end,
                    z_start: opts.safe_z,
                    z_end: opts.target_z,
                    max_ramp_angle_deg: 45.0,
                    style: RampStyle::ZigZag,
                    lateral_amplitude: opts.tool_radius * 0.8,
                },
                cut_state,
            )?)
        }
        EntryMethod::None => Ok(AssemblyResult {
            ops: crate::ops::container::Ops::new(),
            cleared_polygons: vec![],
            start: crate::ops::cut::ToolPose {
                pos: Point3D::new(entry_pt.x, entry_pt.y, opts.target_z),
                heading: 0.0,
            },
            end: crate::ops::cut::ToolPose {
                pos: Point3D::new(entry_pt.x, entry_pt.y, opts.target_z),
                heading: 0.0,
            },
        }),
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
) -> RaygeoResult<AssemblyResult> {
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
        )?
    } else {
        AssemblyResult {
            ops: crate::ops::container::Ops::new(),
            cleared_polygons: vec![],
            start: crate::ops::cut::ToolPose {
                pos: Point3D::new(entry_pt.x, entry_pt.y, opts.target_z),
                heading: 0.0,
            },
            end: crate::ops::cut::ToolPose {
                pos: Point3D::new(entry_pt.x, entry_pt.y, opts.target_z),
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
        )?;

        Ok(chain(helix_result, spiral_result))
    } else {
        Ok(helix_result)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Workplan API: build_entry_workplan
// ═══════════════════════════════════════════════════════════════════════

pub struct EntryWorkplanOptions {
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

#[prof]
pub fn build_entry_workplan(
    opts: &EntryWorkplanOptions,
) -> RaygeoResult<Vec<WorkplanStep>> {
    let regions = find_regions(
        &opts.pocket_boundary,
        &opts.islands,
        opts.tool_radius,
        0.5,
    );

    if regions.is_empty() {
        return Ok(fallback_entry(&opts.pocket_boundary, &opts.islands, opts));
    }

    let mut steps: Vec<WorkplanStep> = Vec::new();
    let tool_diameter = 2.0 * opts.tool_radius;

    for region in &regions {
        if region.r_max >= 2.0 * tool_diameter {
            let helix_r = (opts.tool_radius * 0.8).min(region.r_max * 0.5);

            steps.push(WorkplanStep::HelixPlunge {
                center: region.entry_pt,
                helix_r,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                pitch: opts.plunge_pitch,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            });

            let spiral_max_r =
                (region.r_max - opts.tool_radius - opts.safe_margin)
                    .max(helix_r + 0.01);
            let radial_dist = spiral_max_r - helix_r;
            if radial_dist > 0.0 && opts.step_over > 0.0 {
                let revolutions = radial_dist / opts.step_over;
                steps.push(WorkplanStep::FlatSpiral {
                    center: region.entry_pt,
                    z: opts.target_z,
                    start_radius: helix_r,
                    end_radius: spiral_max_r,
                    revolutions,
                    direction: HelixDirection::Cw,
                    angular_step: opts.angular_step,
                    start_angle: 0.0,
                });
            }
        } else if let Some((start, end)) = find_ramp_carrier(
            &region.polygon,
            &opts.islands,
            opts.tool_radius,
            45.0,
        ) {
            steps.push(WorkplanStep::ToroidalClear {
                carrier: vec![start, end],
                start: Point3D::new(start.x, start.y, opts.safe_z),
                target_z: opts.target_z,
                tool_radius: opts.tool_radius,
                step_over: opts.step_over,
                max_ramp_angle_deg: 45.0,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            });
        } else {
            let (start, end) = ramp_segment_in_region(
                region.entry_pt,
                &region.polygon,
                opts.tool_radius,
            );
            steps.push(WorkplanStep::RampEntry {
                start,
                end,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                max_ramp_angle_deg: 45.0,
                lateral_amplitude: opts.tool_radius * 0.8,
            });
        }
    }

    Ok(steps)
}

fn fallback_entry(
    boundary: &Polygon,
    islands: &[Polygon],
    opts: &EntryWorkplanOptions,
) -> Vec<WorkplanStep> {
    if boundary.is_empty() {
        return Vec::new();
    }

    if let Some((start, end)) =
        find_ramp_carrier(boundary, islands, opts.tool_radius, 45.0)
    {
        vec![WorkplanStep::ToroidalClear {
            carrier: vec![start, end],
            start: Point3D::new(start.x, start.y, opts.safe_z),
            target_z: opts.target_z,
            tool_radius: opts.tool_radius,
            step_over: opts.step_over,
            max_ramp_angle_deg: 45.0,
            direction: HelixDirection::Cw,
            angular_step: opts.angular_step,
        }]
    } else {
        let (entry_pt, _) = find_largest_circle(boundary, islands, 0.1)
            .unwrap_or_else(|| (get_polygon_centroid(boundary), 0.0));
        let (start, end) =
            ramp_segment_in_region(entry_pt, boundary, opts.tool_radius);
        vec![WorkplanStep::RampEntry {
            start,
            end,
            z_start: opts.safe_z,
            z_end: opts.target_z,
            max_ramp_angle_deg: 45.0,
            lateral_amplitude: opts.tool_radius * 0.8,
        }]
    }
}

/// Pick the longest axis-aligned line segment through `entry_pt` whose
/// tool disc stays entirely inside `region`. Tries both X and Y sweeps
/// against the **eroded** region (region eroded by `tool_radius` so the
/// tool disc centred anywhere on the line fits inside the original
/// region). Picks the longest inside sub-segment. Falls back to the
/// AABB-spanning line if erosion yields nothing.
fn ramp_segment_in_region(
    entry_pt: crate::types::Point,
    region: &Polygon,
    tool_radius: f64,
) -> (crate::types::Point, crate::types::Point) {
    use crate::geo::shape::polygon::{offset_polygon, JoinStyle};

    // Erode region by tool_radius (Miter join). Any tool disc centred
    // inside the eroded region fits inside the original region. If
    // erosion collapses (region narrower than 2*tool_radius), fall back
    // to the AABB-spanning line — the caller can shorten or reject.
    let eroded = offset_polygon(region, -tool_radius, JoinStyle::Miter);
    let valid: &[Polygon] = if eroded.is_empty() {
        std::slice::from_ref(region)
    } else {
        &eroded
    };

    let bbox = crate::geo::shape::polygon::get_polygon_bounds(region);

    // Try both axis-aligned orientations through entry_pt spanning the AABB.
    let candidates = [
        (
            crate::types::Point::new(bbox.min.x, entry_pt.y),
            crate::types::Point::new(bbox.max.x, entry_pt.y),
        ),
        (
            crate::types::Point::new(entry_pt.x, bbox.min.y),
            crate::types::Point::new(entry_pt.x, bbox.max.y),
        ),
    ];

    let mut best: Option<(crate::types::Point, crate::types::Point)> = None;
    let mut best_len = 0.0_f64;

    for (p1, p2) in &candidates {
        let clipped =
            crate::geo::algo::clipping::clip_line_segment_with_polygons_2d(
                *p1, *p2, valid,
            );
        for (a, b) in clipped {
            let len = (b.x - a.x).hypot(b.y - a.y);
            if len > best_len {
                best_len = len;
                best = Some((a, b));
            }
        }
    }

    best.unwrap_or_else(|| {
        let bbox = get_polygon_bounds(region);
        longest_line_through_point(entry_pt, bbox)
    })
}

//! Inside-out adaptive wavefront expansion.

use prof_macros::prof;

use crate::ops::cut::ClearedArea;

use crate::error::RaygeoResult;
use crate::geo::shape::polygon::{get_polygon_area, resample_polygon};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::Tracelet;
use crate::ops::cut::ToolPose;
use crate::ops::state::State;
use crate::part::Part;
use crate::types::{Point, Point3D};

const MAX_WAVEFRONT_ITERATIONS: usize = 1000;

/// Options for [`adaptive_wavefronts`].
#[derive(Clone, Debug)]
pub struct AdaptiveWavefrontOptions {
    pub tool_radius: f64,
    pub step_over: f64,
    pub z: f64,
    pub area_tolerance: f64,
    pub precision: f64,
}

/// Inside-out adaptive wavefronts.
///
/// Starting from the cleared area, each iteration expands the frontier
/// (outer boundary) outward by `step_over`, clips to the valid tool
/// area, traces the wavefront, and updates the cleared state.
///
/// Each ring fragment is emitted as `MoveTo` (first point) + `LineTo`
/// (rest), all at height `z`, with `cut_state` applied.
#[prof]
pub fn adaptive_wavefronts(
    _part: &Part,
    trace: &mut Tracelet,
    cleared: &mut ClearedArea,
    opts: &AdaptiveWavefrontOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let mut state_applied = false;

    let mut first_point: Option<Point> = None;
    let mut last_point: Option<Point> = None;

    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let bounded = cleared.bites(
            opts.step_over,
            opts.tool_radius,
            if opts.precision > 0.0 {
                opts.precision
            } else {
                0.01
            },
        );
        if bounded.is_empty() {
            break;
        }

        let new_ring = cleared.cut_fast(&bounded);
        if new_ring.is_empty() {
            continue;
        }

        for frag in &new_ring {
            // Skip fragments that are too small to be meaningful.
            let frag_area = get_polygon_area(frag);
            if frag_area < opts.area_tolerance {
                continue;
            }
            let points: Vec<Point> = if opts.precision > 0.0 {
                resample_polygon(frag, opts.precision)
            } else {
                frag.clone()
            };
            if points.len() < 3 {
                continue;
            }
            if !state_applied {
                trace.apply_state(cut_state);
                state_applied = true;
            }
            if first_point.is_none() {
                first_point = Some(points[0]);
            }
            last_point = Some(points[points.len() - 1]);
            let ring_start = points[0];
            trace.move_to(ring_start.x, ring_start.y, opts.z, None);
            for p in &points[1..] {
                trace.line_to(p.x, p.y, opts.z, None);
            }
            // Close the fragment ring by moving back to the first point.
            trace.line_to(ring_start.x, ring_start.y, opts.z, None);
        }

        let ring_area: f64 = new_ring.iter().map(get_polygon_area).sum();
        // Convergence: stop when the actionable residual (uncleared
        // material inside the tool-centre envelope) drops below the
        // area tolerance, or when this iteration added almost nothing.
        // Using `actionable_remaining` here (instead of comparing
        // `cleared.total_area()` to a precomputed `valid_total_area`)
        // keeps the metric consistent with the polygon convention used
        // by `compute_inset_region` (signed areas: outer CCW
        // positive, island-buffer holes CW negative).  The old
        // comparison broke when `compute_inset_region` switched from
        // `get_polygon_area` (absolute) to `get_polygon_signed_area`
        // because it made `valid_total_area` smaller than
        // `cleared.total_area()` could ever reach, triggering early
        // exit before the wavefront reached the walls.
        if ring_area < opts.area_tolerance
            || cleared.actionable_remaining(opts.tool_radius)
                < opts.area_tolerance
        {
            break;
        }
    }

    let start_pos = first_point.unwrap_or(Point::ZERO);
    let end_pos = last_point.unwrap_or(Point::ZERO);

    Ok(AssemblyMeta {
        cleared_polygons: cleared.fragments().to_vec(),
        start: ToolPose {
            pos: Point3D::new(start_pos.x, start_pos.y, opts.z),
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::new(end_pos.x, end_pos.y, opts.z),
            heading: 0.0,
        },
    })
}

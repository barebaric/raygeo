//! Inside-out adaptive wavefront expansion.

use prof_macros::prof;

use crate::ops::cut::ClearedArea;

use crate::geo::algo::offset::compute_inset_region;
use crate::geo::shape::polygon::{
    get_polygon_area, resample_polygon,
};
use crate::ops::container::Ops;
use crate::ops::state::State;
use crate::types::{Point, Polygon};

const MAX_WAVEFRONT_ITERATIONS: usize = 1000;

/// Options for [`adaptive_wavefronts`].
#[derive(Clone, Debug)]
pub struct AdaptiveWavefrontOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
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
    cleared: &mut ClearedArea,
    opts: &AdaptiveWavefrontOptions,
    cut_state: &State,
) -> Ops {
    let (_, valid_total_area) = compute_inset_region(
        &opts.pocket_boundary,
        opts.tool_radius,
        &opts.islands,
    );

    let mut ops = Ops::new();
    let mut state_applied = false;

    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let bounded = cleared.bites(
            opts.step_over,
            opts.tool_radius,
            if opts.precision > 0.0 { opts.precision } else { 0.01 },
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
                ops.apply_state(cut_state);
                state_applied = true;
            }
            ops.move_to(points[0].x, points[0].y, opts.z, None);
            for p in &points[1..] {
                ops.line_to(p.x, p.y, opts.z, None);
            }
            // Close the fragment ring so the rendering has no visible
            // gap between the last point and the first.
            ops.close_path();
        }

        let ring_area: f64 = new_ring.iter().map(get_polygon_area).sum();
        if ring_area < opts.area_tolerance
            || cleared.total_area() >= valid_total_area - 0.1
        {
            break;
        }
    }

    ops
}

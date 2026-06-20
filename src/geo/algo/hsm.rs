//! HSM (High-Speed Machining) adaptive clearing strategies.

use crate::geo::algo::cleared_area::ClearedArea;
use crate::geo::algo::helix::{generate_helix, HelixDirection, HelixOptions};
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::ramp::{generate_ramp, RampOptions, RampStyle};
use crate::geo::algo::spiral::{generate_spiral, SpiralOptions};
use crate::geo::shape::polygon::{
    get_circle_polygon, get_polygon_area, get_polygon_bounds,
    get_polygon_centroid, get_polygons_group_difference,
    get_segment_swept_polygon, offset_polygon_with_style, JoinStyle,
};
use crate::types::{Point, Point3D, Polygon, Rect};

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

/// Return type of [`adaptive_entry`].
#[derive(Clone, Debug)]
pub struct AdaptiveEntryResult {
    pub toolpath: Vec<Point3D>,
    pub cleared_polygons: Vec<Polygon>,
}

/// Fast central clearing entry.
///
/// Given a pocket boundary (with optional islands), finds the optimal entry
/// pole and generates either:
///
/// - **Helix → Spiral** (wide area, `r_max > 1.5 × tool_radius`): a helical
///   plunge to depth followed by a flat Archimedean spiral.
/// - **ZigZag Ramp** (tight slot, `r_max ≤ 1.5 × tool_radius`): a trochoidal
///   ramp along the longest axis of the slot.
///
/// The result includes the toolpath and the swept polygons (tool-disk or
/// segment-swept) that should be added to the `ClearedArea`.
pub fn adaptive_entry(opts: &AdaptiveEntryOptions) -> AdaptiveEntryResult {
    let (entry_pt, r_max) =
        find_largest_circle(&opts.pocket_boundary, &opts.islands, 0.1)
            .unwrap_or_else(|| {
                let c = get_polygon_centroid(&opts.pocket_boundary);
                (c, 0.0)
            });

    let mut toolpath = Vec::new();

    if r_max > opts.tool_radius * 1.5 {
        let helix_r = (opts.tool_radius * 0.8).min(r_max * 0.5);

        if opts.target_z < opts.safe_z {
            let hp = generate_helix(&HelixOptions {
                center: entry_pt,
                start_radius: helix_r,
                end_radius: helix_r,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                pitch: opts.plunge_pitch,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
                min_revolutions: None,
            });
            toolpath.extend(hp);
        }

        let spiral_max_r =
            (r_max - opts.tool_radius - opts.safe_margin).max(helix_r + 0.01);
        let radial_dist = spiral_max_r - helix_r;

        if radial_dist > 0.0 && opts.step_over > 0.0 {
            let n_revs = radial_dist / opts.step_over;

            let sp = generate_spiral(&SpiralOptions {
                center: entry_pt,
                z: opts.target_z,
                start_radius: helix_r,
                end_radius: spiral_max_r,
                revolutions: n_revs,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            });
            toolpath.extend(sp);
        }

        let disk_r = spiral_max_r + opts.tool_radius;
        let cleared_polygons = vec![get_circle_polygon(entry_pt, disk_r, 64)];

        AdaptiveEntryResult {
            toolpath,
            cleared_polygons,
        }
    } else {
        let bbox = get_polygon_bounds(&opts.pocket_boundary);
        let (start, end) = longest_line_through_point(entry_pt, bbox);

        if opts.target_z < opts.safe_z {
            let rp = generate_ramp(&RampOptions {
                start,
                end,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                max_ramp_angle_deg: 45.0,
                style: RampStyle::ZigZag,
                lateral_amplitude: opts.tool_radius * 0.8,
            });
            toolpath.extend(rp);
        }

        let cleared_polygons =
            get_segment_swept_polygon(start, end, opts.tool_radius);

        AdaptiveEntryResult {
            toolpath,
            cleared_polygons,
        }
    }
}

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
}

/// Return type of [`adaptive_wavefronts`].
#[derive(Clone, Debug)]
pub struct AdaptiveWavefrontResult {
    pub toolpaths: Vec<Vec<Point3D>>,
    pub iterations: usize,
}

/// Inside-out adaptive wavefronts.
///
/// Starting from the cleared area, each iteration expands the frontier
/// (outer boundary) outward by `step_over`, clips to the valid tool
/// area, traces the wavefront, and updates the cleared state.
///
/// Each iteration produces one toolpath — a sequence of 3D points at
/// height `z` tracing the boundaries of all wavefront fragments.
pub fn adaptive_wavefronts(
    cleared: &mut ClearedArea,
    opts: &AdaptiveWavefrontOptions,
) -> AdaptiveWavefrontResult {
    // Compute valid tool centre area = boundary inset minus island buffers.
    let mut valid_tool_area = offset_polygon_with_style(
        &opts.pocket_boundary,
        -opts.tool_radius,
        JoinStyle::Miter,
    );
    if !valid_tool_area.is_empty() && !opts.islands.is_empty() {
        let island_buf: Vec<Polygon> = opts
            .islands
            .iter()
            .flat_map(|isl| {
                offset_polygon_with_style(
                    isl,
                    opts.tool_radius,
                    JoinStyle::Miter,
                )
            })
            .collect();
        if !island_buf.is_empty() {
            valid_tool_area =
                get_polygons_group_difference(&valid_tool_area, &island_buf);
        }
    }
    let valid_total_area: f64 =
        valid_tool_area.iter().map(get_polygon_area).sum();

    let mut toolpaths = Vec::new();

    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let _iter_prof = crate::prof::prof_guard("wf_iteration");

        // Compute the "bites" — the new material reachable by expanding
        // the current frontier outward by step_over, clipped to the
        // valid-tool-area and with already-cleared portions removed.
        let bounded = cleared.bites(opts.step_over, &valid_tool_area, 0.01);
        if bounded.is_empty() {
            break;
        }

        // Add newly reached material to the cleared state.
        let _prof = crate::prof::prof_guard("wf_ring");
        let new_ring = cleared.incorporate(&bounded);
        drop(_prof);
        if new_ring.is_empty() {
            continue;
        }

        // Trace new ring into toolpath with NaN separators between
        // disjoint fragments so matplotlib doesn't draw spurious
        // connecting lines.
        let _prof = crate::prof::prof_guard("wf_trace");
        toolpaths.push(trace_ring(&new_ring, opts.z));
        drop(_prof);

        let ring_area: f64 = new_ring.iter().map(get_polygon_area).sum();
        if ring_area < opts.area_tolerance
            || cleared.total_area() >= valid_total_area - 0.1
        {
            break;
        }
    }

    let n = toolpaths.len();
    AdaptiveWavefrontResult {
        toolpaths,
        iterations: n,
    }
}

/// Trace a set of polygon fragments into a single toolpath with NaN
/// separators between fragments (so downstream renderers don't draw
/// spurious connecting lines).
fn trace_ring(ring: &[Polygon], z: f64) -> Vec<Point3D> {
    let mut path = Vec::new();
    for poly in ring {
        if !path.is_empty() {
            path.push(Point3D::new(f64::NAN, f64::NAN, z));
        }
        for p in poly {
            path.push(Point3D::new(p.x, p.y, z));
        }
    }
    path
}

/// Find a line segment through `pt` that spans the bounding box,
/// choosing the longest axis.
fn longest_line_through_point(pt: Point, bbox: Rect) -> (Point, Point) {
    let w = bbox.max.x - bbox.min.x;
    let h = bbox.max.y - bbox.min.y;
    if w >= h {
        (Point::new(bbox.min.x, pt.y), Point::new(bbox.max.x, pt.y))
    } else {
        (Point::new(pt.x, bbox.min.y), Point::new(pt.x, bbox.max.y))
    }
}

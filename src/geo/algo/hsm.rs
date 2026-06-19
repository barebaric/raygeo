//! HSM (High-Speed Machining) adaptive clearing strategies.

use crate::geo::algo::helix::{generate_helix, HelixDirection, HelixOptions};
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::ramp::{generate_ramp, RampOptions, RampStyle};
use crate::geo::algo::spiral::{generate_spiral, SpiralOptions};
use crate::geo::shape::polygon::{
    get_circle_polygon, get_polygon_bounds, get_polygon_centroid,
    get_segment_swept_polygon,
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

/// Find a line segment through `pt` that spans the bounding box,
/// choosing the longest axis.
fn longest_line_through_point(pt: Point, bbox: Rect) -> (Point, Point) {
    let Rect(xmin, ymin, xmax, ymax) = bbox;
    let w = xmax - xmin;
    let h = ymax - ymin;
    if w >= h {
        (Point::new(xmin, pt.y), Point::new(xmax, pt.y))
    } else {
        (Point::new(pt.x, ymin), Point::new(pt.x, ymax))
    }
}

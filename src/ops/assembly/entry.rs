//! Central clearing entry strategies (helix→spiral / zigzag ramp).

use prof_macros::prof;

use crate::geo::algo::helix::{
    generate_helix_3d, HelixDirection, HelixOptions,
};
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::ramp::{generate_ramp_3d, RampOptions, RampStyle};
use crate::geo::algo::spiral::{generate_spiral_3d, SpiralOptions};
use crate::geo::shape::line::longest_line_through_point;
use crate::geo::shape::polygon::{
    get_circle_polygon, get_polygon_bounds, get_polygon_centroid,
    get_segment_swept_polygon,
};
use crate::ops::container::Ops;
use crate::ops::state::State;
use crate::types::{Point3D, Polygon};

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
    pub ops: Ops,
    pub cleared_polygons: Vec<Polygon>,
}

/// Fast central clearing entry.
///
/// Given a pocket boundary (with optional islands), finds the optimal
/// entry pole and generates either:
///
/// - **Helix → Spiral** (wide area): helical plunge to depth followed by
///   a flat Archimedean spiral.
/// - **ZigZag Ramp** (tight slot): a trochoidal ramp along the longest
///   axis of the slot.
///
/// The result includes the Ops (with `cut_state` applied) and the swept
/// polygons that should be added to the [`ClearedArea`].
pub fn adaptive_entry(
    opts: &AdaptiveEntryOptions,
    cut_state: &State,
) -> AdaptiveEntryResult {
    let (entry_pt, r_max) =
        find_largest_circle(&opts.pocket_boundary, &opts.islands, 0.1)
            .unwrap_or_else(|| {
                let c = get_polygon_centroid(&opts.pocket_boundary);
                (c, 0.0)
            });

    let mut toolpath: Vec<Point3D> = Vec::new();

    if r_max > opts.tool_radius * 1.5 {
        let helix_r = (opts.tool_radius * 0.8).min(r_max * 0.5);

        if opts.target_z < opts.safe_z {
            let hp = generate_helix_3d(&HelixOptions {
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

            let start_angle = if let Some(last) = toolpath.last() {
                (last.y - entry_pt.y).atan2(last.x - entry_pt.x)
            } else {
                0.0
            };

            let sp = generate_spiral_3d(&SpiralOptions {
                center: entry_pt,
                z: opts.target_z,
                start_radius: helix_r,
                end_radius: spiral_max_r,
                revolutions: n_revs,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
                start_angle,
            });
            toolpath.extend(sp);
        }

        // Final circular pass at the outer radius to smooth out the
        // scalloped boundary left by the Archimedean spiral.
        if !toolpath.is_empty() {
            let last = *toolpath.last().unwrap();
            let start_a = (last.y - entry_pt.y).atan2(last.x - entry_pt.x);
            let n_circ = ((2.0 * std::f64::consts::PI / opts.angular_step)
                .ceil() as usize)
                .max(8);
            for i in 1..=n_circ {
                let a = start_a
                    - i as f64 * 2.0 * std::f64::consts::PI / n_circ as f64;
                toolpath.push(Point3D::new(
                    entry_pt.x + spiral_max_r * a.cos(),
                    entry_pt.y + spiral_max_r * a.sin(),
                    opts.target_z,
                ));
            }
        }

        let disk_r = spiral_max_r;
        let cleared_polygons = vec![get_circle_polygon(entry_pt, disk_r, 64)];

        AdaptiveEntryResult {
            ops: points_to_ops(&toolpath, cut_state),
            cleared_polygons,
        }
    } else {
        let bbox = get_polygon_bounds(&opts.pocket_boundary);
        let (start, end) = longest_line_through_point(entry_pt, bbox);

        let lateral_amplitude = opts.tool_radius * 0.8;

        if opts.target_z < opts.safe_z {
            let rp = generate_ramp_3d(&RampOptions {
                start,
                end,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                max_ramp_angle_deg: 45.0,
                style: RampStyle::ZigZag,
                lateral_amplitude,
            });
            toolpath.extend(rp);
        }

        let cleared_polygons =
            get_segment_swept_polygon(start, end, lateral_amplitude);

        AdaptiveEntryResult {
            ops: points_to_ops(&toolpath, cut_state),
            cleared_polygons,
        }
    }
}

/// Build Ops from a 3-D polyline: apply state, MoveTo first point,
/// LineTo the rest.
#[prof]
fn points_to_ops(path: &[Point3D], cut_state: &State) -> Ops {
    let mut ops = Ops::new();
    if path.is_empty() {
        return ops;
    }
    ops.apply_state(cut_state);
    ops.move_to(path[0].x, path[0].y, path[0].z, None);
    for p in &path[1..] {
        ops.line_to(p.x, p.y, p.z, None);
    }
    ops
}

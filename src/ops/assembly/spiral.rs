//! Spiral entry path generation.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::spiral::{
    generate_spiral_3d, SpiralOptions as GeoSpiralOptions,
};
use crate::geo::shape::polygon::get_circle_polygon;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::write_polyline;
use crate::ops::assembly::Tracelet;
use crate::ops::cut::Part;
use crate::ops::cut::ToolPose;
use crate::ops::state::State;
use crate::types::{Point, Point3D};

/// Options for generating a flat spiral entry path.
#[derive(Clone, Debug)]
pub struct SpiralOptions {
    pub center: Point,
    pub z: f64,
    pub start_radius: f64,
    pub end_radius: f64,
    pub revolutions: f64,
    pub direction: HelixDirection,
    pub angular_step: f64,
    pub start_angle: f64,
}

/// Generate a flat spiral entry path followed by a smoothing circular pass.
///
/// Calls the geo-layer spiral generator, appends a full-circle pass at
/// `end_radius` to smooth the scalloped boundary, and wraps the result
/// into an [`AssemblyResult`].
#[prof]
pub fn generate_spiral(
    _part: &Part,
    trace: &mut Tracelet,
    opts: &SpiralOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let mut path = generate_spiral_3d(&GeoSpiralOptions {
        center: opts.center,
        z: opts.z,
        start_radius: opts.start_radius,
        end_radius: opts.end_radius,
        revolutions: opts.revolutions,
        direction: opts.direction,
        angular_step: opts.angular_step,
        start_angle: opts.start_angle,
    });

    // Final circular pass at the outer radius to smooth out the
    // scalloped boundary left by the Archimedean spiral.
    if !path.is_empty() {
        let last = *path.last().unwrap();
        let start_a = (last.y - opts.center.y).atan2(last.x - opts.center.x);
        let dir_sign = match opts.direction {
            HelixDirection::Cw => -1.0,
            HelixDirection::Ccw => 1.0,
        };
        let n_circ = ((2.0 * std::f64::consts::PI / opts.angular_step).ceil()
            as usize)
            .max(8);
        for i in 1..=n_circ {
            let a = start_a
                + i as f64 * 2.0 * std::f64::consts::PI / n_circ as f64
                    * dir_sign;
            path.push(Point3D::new(
                opts.center.x + opts.end_radius * a.cos(),
                opts.center.y + opts.end_radius * a.sin(),
                opts.z,
            ));
        }
    }

    let start = if path.is_empty() {
        ToolPose {
            pos: Point3D::new(opts.center.x, opts.center.y, opts.z),
            heading: 0.0,
        }
    } else {
        ToolPose {
            pos: path[0],
            heading: spiral_heading(&path, 0),
        }
    };

    let end = if path.is_empty() {
        ToolPose {
            pos: Point3D::new(opts.center.x, opts.center.y, opts.z),
            heading: 0.0,
        }
    } else {
        let n = path.len();
        ToolPose {
            pos: path[n - 1],
            heading: spiral_heading(&path, n - 1),
        }
    };

    let cleared_polygons = if path.is_empty() {
        vec![]
    } else {
        vec![get_circle_polygon(opts.center, opts.end_radius, 64)]
    };

    write_polyline(trace, &path, true, Some(cut_state));
    Ok(AssemblyMeta {
        cleared_polygons,
        start,
        end,
    })
}

/// Compute the tangent heading at index `i` in the spiral path.
fn spiral_heading(path: &[Point3D], i: usize) -> f64 {
    if i + 1 < path.len() {
        let dx = path[i + 1].x - path[i].x;
        let dy = path[i + 1].y - path[i].y;
        if dx.abs() > 1e-12 || dy.abs() > 1e-12 {
            return dy.atan2(dx);
        }
    }
    0.0
}

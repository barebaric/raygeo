//! Toroidal (trochoidal) slot entry path generation.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::trochoid::{get_trochoid_along_3d, TrochoidOptions};
use crate::ops::assembly::result::AssemblyResult;
use crate::ops::container::Ops;
use crate::ops::cut::ToolPose;
use crate::ops::state::State;
use crate::types::{Point, Point3D, Polygon};

/// Options for generating a toroidal (trochoidal) path along a carrier.
#[derive(Clone, Debug)]
pub struct ToroidOptions {
    pub carrier: Vec<Point>,
    pub tool_radius: f64,
    pub step_over: f64,
    pub target_z: f64,
    pub direction: HelixDirection,
    pub angular_step: f64,
}

/// Generate a toroidal entry path along a carrier polyline.
///
/// Calls the geo-layer trochoid generator and wraps the result into an
/// [`AssemblyResult`]. The cleared polygon is the Minkowski sum of the
/// carrier with a disk of `tool_radius`.
#[prof]
pub fn generate_toroid(
    opts: &ToroidOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyResult> {
    let path = get_trochoid_along_3d(
        &opts.carrier,
        &TrochoidOptions {
            diameter: opts.tool_radius * 2.0,
            engagement_angle_deg: 30.0,
            step_over_ratio: opts.step_over / (opts.tool_radius * 2.0),
            min_loop_radius: opts.tool_radius * 0.3,
            z: opts.target_z,
        },
    );

    let start = if path.is_empty() {
        ToolPose {
            pos: opts
                .carrier
                .first()
                .map(|p| Point3D::new(p.x, p.y, opts.target_z))
                .unwrap_or(Point3D::ZERO),
            heading: 0.0,
        }
    } else {
        ToolPose {
            pos: path[0],
            heading: toroid_heading(&path, 0, opts.direction),
        }
    };

    let end = if path.is_empty() {
        ToolPose {
            pos: opts
                .carrier
                .last()
                .map(|p| Point3D::new(p.x, p.y, opts.target_z))
                .unwrap_or(Point3D::ZERO),
            heading: 0.0,
        }
    } else {
        let n = path.len();
        ToolPose {
            pos: path[n - 1],
            heading: toroid_heading(&path, n - 1, opts.direction),
        }
    };

    let cleared_polygons = if opts.carrier.len() >= 2 {
        let mut poly: Polygon = Vec::new();
        for &p in &opts.carrier {
            poly.push(p);
        }
        // Sweep the carrier with tool_radius
        let swept = swept_polygon_from_carrier(&opts.carrier, opts.tool_radius);
        vec![swept]
    } else {
        vec![]
    };

    Ok(AssemblyResult {
        ops: Ops::from_polyline(&path, true, Some(cut_state)),
        cleared_polygons,
        start,
        end,
    })
}

/// Build a swept polygon around a carrier polyline at tool radius.
fn swept_polygon_from_carrier(carrier: &[Point], tool_radius: f64) -> Polygon {
    let mut poly = Polygon::new();
    if carrier.is_empty() {
        return poly;
    }
    for p in carrier {
        poly.push(Point::new(p.x + tool_radius, p.y + tool_radius));
    }
    for p in carrier.iter().rev() {
        poly.push(Point::new(p.x - tool_radius, p.y - tool_radius));
    }
    poly
}

/// Compute the tangent heading at index `i` in the toroid path.
fn toroid_heading(
    path: &[Point3D],
    i: usize,
    _direction: HelixDirection,
) -> f64 {
    if i + 1 < path.len() {
        let dx = path[i + 1].x - path[i].x;
        let dy = path[i + 1].y - path[i].y;
        if dx.abs() > 1e-12 || dy.abs() > 1e-12 {
            return dy.atan2(dx);
        }
    }
    0.0
}

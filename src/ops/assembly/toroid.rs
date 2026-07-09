//! Toroidal (trochoidal) slot entry path generation.

use prof_macros::prof;

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::trochoid::{
    get_trochoid_along_3d, get_trochoid_along_3d_ramped, TrochoidOptions,
    TrochoidOptionsRamped,
};
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
        trace: None,
    })
}

/// Options for generating a ramp-down toroidal clear path along a carrier.
#[derive(Clone, Debug)]
pub struct ToroidalClearOptions {
    pub carrier: Vec<Point>,
    pub start: Point3D,
    pub target_z: f64,
    pub tool_radius: f64,
    pub step_over: f64,
    pub max_ramp_angle_deg: f64,
    pub direction: HelixDirection,
    pub angular_step: f64,
}

/// Generate a ramp-down toroidal clear path that descends Z continuously
/// along the carrier's arc-length, zig-zagging back-and-forth along the
/// carrier until `target_z` is reached, then emits one final full forward
/// pass at constant `target_z`.
#[prof]
pub fn generate_toroidal_clear(
    opts: &ToroidalClearOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyResult> {
    if opts.carrier.len() < 2 {
        let pos = opts
            .carrier
            .first()
            .map(|p| Point3D::new(p.x, p.y, opts.target_z))
            .unwrap_or(Point3D::ZERO);
        return Ok(AssemblyResult {
            ops: Ops::new(),
            cleared_polygons: vec![],
            start: ToolPose { pos, heading: 0.0 },
            end: ToolPose { pos, heading: 0.0 },
            trace: None,
        });
    }

    let delta_z = (opts.start.z - opts.target_z).max(0.0);

    let l_pass: f64 = opts
        .carrier
        .windows(2)
        .map(|w| (w[1] - w[0]).length())
        .sum();

    if l_pass < 1e-12 {
        let pos =
            Point3D::new(opts.carrier[0].x, opts.carrier[0].y, opts.target_z);
        return Ok(AssemblyResult {
            ops: Ops::new(),
            cleared_polygons: vec![],
            start: ToolPose { pos, heading: 0.0 },
            end: ToolPose { pos, heading: 0.0 },
            trace: None,
        });
    }

    let l_min = if delta_z > 0.0 {
        delta_z / (opts.max_ramp_angle_deg.to_radians().tan())
    } else {
        0.0
    };

    if delta_z < 1e-12 || l_min < 1e-12 {
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
        return build_toroidal_result(&path, opts, cut_state);
    }

    let mut current_z = opts.start.z;
    let mut forward = true;
    let mut full_path: Vec<Point3D> = Vec::new();
    let original_delta_z = opts.start.z - opts.target_z;
    let reversed_carrier: Vec<Point> =
        opts.carrier.iter().copied().rev().collect();

    for _pass in 0..10000 {
        if current_z <= opts.target_z + 1e-9 {
            break;
        }

        let delta_z_remaining = current_z - opts.target_z;
        let d_per_pass =
            delta_z_remaining.min(original_delta_z * (l_pass / l_min));
        let z_pass_start = current_z;
        let z_pass_end = (current_z - d_per_pass).max(opts.target_z);

        let current_carrier: &[Point] = if forward {
            &opts.carrier
        } else {
            &reversed_carrier
        };

        let pass_pts = get_trochoid_along_3d_ramped(
            current_carrier,
            &TrochoidOptionsRamped {
                diameter: opts.tool_radius * 2.0,
                engagement_angle_deg: 30.0,
                step_over_ratio: opts.step_over / (opts.tool_radius * 2.0),
                min_loop_radius: opts.tool_radius * 0.3,
                z_start: z_pass_start,
                z_end: z_pass_end,
            },
        );

        full_path.extend(pass_pts);
        current_z = z_pass_end;
        forward = !forward;
    }

    if current_z > opts.target_z + 1e-9 {
        return Err(RaygeoError::InternalError(
            "toroidal clear exceeded maximum number of passes".to_string(),
        ));
    }

    let final_path = get_trochoid_along_3d(
        &opts.carrier,
        &TrochoidOptions {
            diameter: opts.tool_radius * 2.0,
            engagement_angle_deg: 30.0,
            step_over_ratio: opts.step_over / (opts.tool_radius * 2.0),
            min_loop_radius: opts.tool_radius * 0.3,
            z: opts.target_z,
        },
    );

    full_path.extend(final_path);
    build_toroidal_result(&full_path, opts, cut_state)
}

/// Build an [`AssemblyResult`] from a full 3D trochoid path.
fn build_toroidal_result(
    path: &[Point3D],
    opts: &ToroidalClearOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyResult> {
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
            heading: toroid_heading(path, 0, opts.direction),
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
            heading: toroid_heading(path, n - 1, opts.direction),
        }
    };

    let cleared_polygons = if opts.carrier.len() >= 2 {
        vec![swept_polygon_from_carrier(&opts.carrier, opts.tool_radius)]
    } else {
        vec![]
    };

    Ok(AssemblyResult {
        ops: Ops::from_polyline(path, true, Some(cut_state)),
        cleared_polygons,
        start,
        end,
        trace: None,
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

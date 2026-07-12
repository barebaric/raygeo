/// Back-and-forth slot clearing path generation.
///
/// Emits a forward pass along every carrier point followed immediately by
/// a backward pass in reverse, both at constant `target_z`. The cleared
/// polygon is the carrier swept by `tool_radius` (perpendicular offset
/// along the average carrier direction).
///
/// # Algorithm
///
/// 1. Build the forward pass through all carrier points at `target_z`.
/// 2. Build the backward pass through all carrier points in reverse at `target_z`.
/// 3. Concatenate forward + backward into a single path.
/// 4. Compute the swept polygon from the carrier.
/// 5. Produce the start/end poses with tangent headings.
use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::trace_utils as tu;
use crate::ops::assembly::write_polyline;
use crate::ops::assembly::Tracelet;
use crate::ops::part::Part;
use crate::ops::state::State;
use crate::ops::types::ToolPose;
use crate::types::{Point, Point3D, Polygon};

/// Options for generating a back-and-forth slot pass.
#[derive(Clone, Debug)]
pub struct SlotOptions {
    pub carrier: Vec<Point>,
    pub tool_radius: f64,
    pub target_z: f64,
}

/// Generate a back-and-forth slot clearing path along a carrier.
///
/// Returns an [`AssemblyResult`] with a forward-then-backward linear path
/// at constant `target_z`. The cleared polygon is the carrier swept by
/// `tool_radius`.
#[prof]
pub fn generate_slot(
    part: &mut Part,
    trace: &mut Tracelet,
    opts: &SlotOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    if opts.carrier.len() < 2 {
        let pos = opts
            .carrier
            .first()
            .map(|p| Point3D::new(p.x, p.y, opts.target_z))
            .unwrap_or(Point3D::ZERO);
        return Ok(AssemblyMeta {
            start: ToolPose { pos, heading: 0.0 },
            end: ToolPose { pos, heading: 0.0 },
        });
    }

    let z = opts.target_z;

    // Forward pass through all carrier points.
    let n = opts.carrier.len();
    let mut path: Vec<Point3D> = Vec::with_capacity(n * 2);
    for p in &opts.carrier {
        path.push(Point3D::new(p.x, p.y, z));
    }
    // Backward pass in reverse.
    for p in opts.carrier.iter().rev() {
        path.push(Point3D::new(p.x, p.y, z));
    }

    let start = ToolPose {
        pos: path[0],
        heading: tu::path_heading(&path, 0),
    };
    let end = {
        let n = path.len();
        ToolPose {
            pos: path[n - 1],
            heading: tu::path_heading(&path, n - 1),
        }
    };

    let cleared_polygons = if opts.carrier.len() >= 2 {
        vec![swept_polygon_from_carrier(&opts.carrier, opts.tool_radius)]
    } else {
        vec![]
    };

    write_polyline(trace, &path, true, Some(cut_state));
    part.cleared.cut(&cleared_polygons);
    Ok(AssemblyMeta { start, end })
}

/// Build a swept polygon around a carrier polyline at tool radius.
///
/// Uses the average direction (first-to-last point) to compute a
/// perpendicular offset, producing a bounding polygon for the swept area.
/// Includes semicircular caps at both ends by extending `tool_radius` along
/// the carrier direction at the start and end.
fn swept_polygon_from_carrier(carrier: &[Point], tool_radius: f64) -> Polygon {
    let mut poly = Polygon::new();
    let n = carrier.len();
    if n == 0 {
        return poly;
    }

    // Average direction from first to last.
    let dx = carrier[n - 1].x - carrier[0].x;
    let dy = carrier[n - 1].y - carrier[0].y;
    let len = (dx * dx + dy * dy).sqrt();
    let (dir_x, dir_y, perp_x, perp_y) = if len > 1e-12 {
        (
            dx / len,
            dy / len,
            -dy / len * tool_radius,
            dx / len * tool_radius,
        )
    } else {
        (1.0, 0.0, 0.0, tool_radius)
    };

    let start_ext_x = -dir_x * tool_radius;
    let start_ext_y = -dir_y * tool_radius;
    let end_ext_x = dir_x * tool_radius;
    let end_ext_y = dir_y * tool_radius;

    // Right side (forward), including caps.
    poly.push(Point::new(
        carrier[0].x + start_ext_x + perp_x,
        carrier[0].y + start_ext_y + perp_y,
    ));
    for p in carrier {
        poly.push(Point::new(p.x + perp_x, p.y + perp_y));
    }
    poly.push(Point::new(
        carrier[n - 1].x + end_ext_x + perp_x,
        carrier[n - 1].y + end_ext_y + perp_y,
    ));

    // Left side (reverse), including caps.
    poly.push(Point::new(
        carrier[n - 1].x + end_ext_x - perp_x,
        carrier[n - 1].y + end_ext_y - perp_y,
    ));
    for p in carrier.iter().rev() {
        poly.push(Point::new(p.x - perp_x, p.y - perp_y));
    }
    poly.push(Point::new(
        carrier[0].x + start_ext_x - perp_x,
        carrier[0].y + start_ext_y - perp_y,
    ));
    poly
}

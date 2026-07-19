//! Helical entry path generation.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::helix::{
    generate_helix_3d, HelixDirection, HelixOptions as GeoHelixOptions,
};
use crate::geo::shape::polygon::get_circle_polygon;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::trace_utils as tu;
use crate::ops::assembly::write_polyline;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::part::Part;
use crate::ops::state::State;
use crate::ops::types::ToolPose;
use crate::types::{Point, Point3D};

/// Spec for the helix assembler.
///
/// Carries the parameters needed by [`generate_helix`] and the workplan
/// executor's [`WorkplanStep::HelixPlunge`](crate::cnc::machining::plan::WorkplanStep::HelixPlunge)
/// step.
#[derive(Clone, Debug)]
pub struct HelixSpec {
    pub center: Point,
    pub start_radius: f64,
    pub z_start: f64,
    pub z_end: f64,
    pub pitch: f64,
    pub direction: HelixDirection,
    pub angular_step: f64,
}

impl Assembler for HelixSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "helix: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_helix(ctx.part, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "helix: done");
        Ok(meta)
    }

    fn name(&self) -> &'static str {
        "helix"
    }
}

/// Generate a helical entry path.
///
/// Calls the geo-layer helix generator and wraps the result into an
/// [`AssemblyResult`] with heading (tangent direction) at first/last point
/// and a disk-shaped cleared polygon at the helix radius.
#[prof]
pub fn generate_helix(
    part: &mut Part,
    trace: &mut Tracelet,
    opts: &HelixSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let path = generate_helix_3d(&GeoHelixOptions {
        center: opts.center,
        start_radius: opts.start_radius,
        end_radius: opts.start_radius,
        z_start: opts.z_start,
        z_end: opts.z_end,
        pitch: opts.pitch,
        direction: opts.direction,
        angular_step: opts.angular_step,
        min_revolutions: None,
    });

    let start = if path.is_empty() {
        ToolPose {
            pos: Point3D::new(opts.center.x, opts.center.y, opts.z_start),
            heading: 0.0,
        }
    } else {
        ToolPose {
            pos: path[0],
            heading: compute_heading(&path, 0, &opts.center, opts.direction),
        }
    };

    let end = if path.is_empty() {
        ToolPose {
            pos: Point3D::new(opts.center.x, opts.center.y, opts.z_end),
            heading: 0.0,
        }
    } else {
        let n = path.len();
        ToolPose {
            pos: path[n - 1],
            heading: compute_heading(
                &path,
                n - 1,
                &opts.center,
                opts.direction,
            ),
        }
    };

    let cleared_polygons = if path.is_empty() {
        vec![]
    } else {
        vec![get_circle_polygon(opts.center, opts.start_radius, 64)]
    };

    write_polyline(trace, &path, true, Some(cut_state));
    part.cleared.cut(&cleared_polygons);
    Ok(AssemblyMeta { start, end })
}

/// Compute the tangent heading at index `i` in the helix path.
///
/// Uses finite-difference with the next point in the path when available,
/// falling back to the analytic tangent perpendicular to the radius vector
/// (for single-point paths).
fn compute_heading(
    path: &[Point3D],
    i: usize,
    center: &Point,
    direction: HelixDirection,
) -> f64 {
    let fd = tu::path_heading(path, i);
    if fd != 0.0 || i + 1 < path.len() {
        return fd;
    }
    // Analytic tangent: perpendicular to radius vector.
    let rx = path[i].x - center.x;
    let ry = path[i].y - center.y;
    if rx.abs() < 1e-12 && ry.abs() < 1e-12 {
        return 0.0;
    }
    match direction {
        HelixDirection::Cw => (-ry).atan2(rx),
        HelixDirection::Ccw => ry.atan2(-rx),
    }
}

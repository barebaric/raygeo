//! Ramp entry path generation.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::ramp::{
    generate_ramp_3d, RampOptions as GeoRampOptions, RampStyle,
};
use crate::geo::shape::polygon::get_segment_swept_polygon;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::trace_utils as tu;
use crate::ops::assembly::write_polyline;
use crate::ops::assembly::AssemblyOutput;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::cache::Cacheable;
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::ops::types::ToolPose;
use crate::types::{Point, Point3D};

/// Spec for the ramp assembler.
#[derive(Clone, Debug)]
pub struct RampSpec {
    pub start: Point,
    pub end: Point,
    pub z_start: f64,
    pub z_end: f64,
    pub max_ramp_angle_deg: f64,
    pub style: RampStyle,
    pub lateral_amplitude: f64,
}

impl Assembler for RampSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "ramp: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_ramp(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "ramp: done");
        Ok(meta)
    }

    fn name(&self) -> &'static str {
        "ramp"
    }
}

impl Cacheable<AssemblyOutput> for RampSpec {}

/// Generate a ramp entry path.
///
/// Calls the geo-layer ramp generator and wraps the result into an
/// [`AssemblyResult`] with a segment-swept cleared polygon.
#[prof]
pub fn generate_ramp(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &RampSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let path = generate_ramp_3d(&GeoRampOptions {
        start: opts.start,
        end: opts.end,
        z_start: opts.z_start,
        z_end: opts.z_end,
        max_ramp_angle_deg: opts.max_ramp_angle_deg,
        style: opts.style,
        lateral_amplitude: opts.lateral_amplitude,
    });

    let start = if path.is_empty() {
        ToolPose {
            pos: Point3D::new(opts.start.x, opts.start.y, opts.z_start),
            heading: 0.0,
        }
    } else {
        ToolPose {
            pos: path[0],
            heading: tu::path_heading(&path, 0),
        }
    };

    let end = if path.is_empty() {
        ToolPose {
            pos: Point3D::new(opts.end.x, opts.end.y, opts.z_end),
            heading: 0.0,
        }
    } else {
        let n = path.len();
        ToolPose {
            pos: path[n - 1],
            heading: tu::path_heading(&path, n - 1),
        }
    };

    let cleared_polygons =
        get_segment_swept_polygon(opts.start, opts.end, opts.lateral_amplitude);

    write_polyline(trace, &path, true, Some(cut_state));
    face.cleared.cut(&cleared_polygons);
    Ok(AssemblyMeta { start, end })
}

//! Ramp entry path generation.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::ramp::{
    generate_ramp_3d, RampOptions as GeoRampOptions, RampStyle,
};
use crate::geo::shape::polygon::get_segment_swept_polygon;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::write_polyline;
use crate::ops::assembly::Tracelet;
use crate::ops::part::Part;
use crate::ops::state::State;
use crate::ops::types::ToolPose;
use crate::types::{Point, Point3D};

/// Options for generating a ramp entry path.
#[derive(Clone, Debug)]
pub struct RampOptions {
    pub start: Point,
    pub end: Point,
    pub z_start: f64,
    pub z_end: f64,
    pub max_ramp_angle_deg: f64,
    pub style: RampStyle,
    pub lateral_amplitude: f64,
}

/// Generate a ramp entry path.
///
/// Calls the geo-layer ramp generator and wraps the result into an
/// [`AssemblyResult`] with a segment-swept cleared polygon.
#[prof]
pub fn generate_ramp(
    part: &mut Part,
    trace: &mut Tracelet,
    opts: &RampOptions,
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
            heading: ramp_heading(&path, 0),
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
            heading: ramp_heading(&path, n - 1),
        }
    };

    let cleared_polygons =
        get_segment_swept_polygon(opts.start, opts.end, opts.lateral_amplitude);

    write_polyline(trace, &path, true, Some(cut_state));
    part.cleared.cut(&cleared_polygons);
    Ok(AssemblyMeta { start, end })
}

/// Compute the tangent heading at index `i` in the ramp path.
fn ramp_heading(path: &[crate::types::Point3D], i: usize) -> f64 {
    if i + 1 < path.len() {
        let dx = path[i + 1].x - path[i].x;
        let dy = path[i + 1].y - path[i].y;
        if dx.abs() > 1e-12 || dy.abs() > 1e-12 {
            return dy.atan2(dx);
        }
    }
    0.0
}

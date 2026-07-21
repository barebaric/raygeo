//! Spiral entry path generation.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::spiral::{
    generate_spiral_3d, SpiralOptions as GeoSpiralOptions,
};
use crate::geo::shape::polygon::get_circle_polygon;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::trace_utils as tu;
use crate::ops::assembly::write_polyline;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::types::{Point, Point3D};

/// Spec for the spiral assembler.
///
/// Mirrors the parameter list of [`generate_spiral`]. Held as
/// `Box<dyn Assembler>` by callers that drive the trait.
#[derive(Clone, Debug)]
pub struct SpiralSpec {
    pub center: Point,
    pub z: f64,
    pub start_radius: f64,
    pub end_radius: f64,
    pub revolutions: f64,
    pub direction: HelixDirection,
    pub angular_step: f64,
    pub start_angle: f64,
}

impl Assembler for SpiralSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "spiral: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_spiral(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "spiral: done");
        Ok(meta)
    }

    fn name(&self) -> &'static str {
        "spiral"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Generate a flat spiral entry path followed by a smoothing circular pass.
///
/// Calls the geo-layer spiral generator, appends a full-circle pass at
/// `end_radius` to smooth the scalloped boundary, and wraps the result
/// into an [`AssemblyResult`].
#[prof]
pub fn generate_spiral(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &SpiralSpec,
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

    let fallback = Point3D::new(opts.center.x, opts.center.y, opts.z);
    let (start, end) = tu::start_end_poses(&path, fallback);

    let cleared_polygons = if path.is_empty() {
        vec![]
    } else {
        vec![get_circle_polygon(opts.center, opts.end_radius, 64)]
    };

    write_polyline(trace, &path, true, Some(cut_state));
    face.cleared.cut(&cleared_polygons);
    Ok(AssemblyMeta { start, end })
}

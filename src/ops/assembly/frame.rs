//! Frame assembler: generate a rectangular frame around the part
//! boundary.
//!
//! Pure-Rust core. The Python `frame` pyfunction in
//! `crate::python::ops::assembly::frame` is a thin wrapper that calls
//! [`assemble_frame`] and packs the result into a
//! [`PyAssemblyResult`](crate::python::ops::assembly::result::PyAssemblyResult).
//!
//! The [`FrameSpec`] struct implements the [`Assembler`] trait so
//! callers can dispatch to it without knowing the concrete parameter
//! set.

use crate::error::RaygeoResult;
use crate::geo::algo::offset::grow_geometry;
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon3d::fillet_polyline_3d;
use crate::geo::types::Point3D;
use crate::ops::assembly::contour::compute_total_offset;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{wrap_vector_outline, AssembleCtx, Assembler};
use crate::ops::container::Ops;
use crate::ops::types::ToolPose;

/// Spec for the frame assembler.
///
/// Mirrors the parameter list of [`assemble_frame`]. Held as
/// `Box<dyn Assembler>` by callers that drive the trait.
#[derive(Clone, Debug)]
pub struct FrameSpec {
    pub offset_mm: f64,
    pub cut_side: String,
    pub corner_radius: f64,
}

impl Assembler for FrameSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "frame: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let (ops, meta) = assemble_frame(
            ctx.size_mm,
            self.offset_mm,
            &self.cut_side,
            self.corner_radius,
        )
        .map_err(|e| e.to_string())?;
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        ctx.trace.append_ops(&ops);
        ctx.callbacks.report_progress(1.0, "frame: done");
        Ok(meta)
    }

    fn is_scalable(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "frame"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Generate a rectangular frame matching the part's `size_mm`,
/// optionally offset by offset / cut-side and with rounded corners.
///
/// `corner_radius` is the radius of the rounded corners in mm. It is
/// clamped to what fits the final frame, so an oversized value
/// degrades gracefully instead of producing an invalid path.
///
/// Returns `(Ops, AssemblyMeta)` with zero start/end tool poses (the
/// frame is a closed rectangle with no entry/exit move).
pub fn assemble_frame(
    size_mm: (f64, f64),
    offset_mm: f64,
    cut_side: &str,
    corner_radius: f64,
) -> RaygeoResult<(Ops, AssemblyMeta)> {
    let (w, h) = size_mm;
    if w <= 0.0 || h <= 0.0 {
        return Err(crate::error::RaygeoError::ContourError(
            "Part has invalid or zero size".to_string(),
        ));
    }

    let total_offset = compute_total_offset(offset_mm, cut_side);

    // The requested radius applies to the *final* (offset) frame, but
    // rounding happens before the offset, so translate it back and
    // clamp it to what the offset frame can accommodate.
    let final_half = (w.min(h) / 2.0 + total_offset).max(0.0);
    let final_radius = corner_radius.clamp(0.0, final_half);
    let base_radius = (final_radius - total_offset).max(0.0);

    let mut geo =
        Geometry::from_points(&rectangle_outline(w, h, base_radius), false);

    if total_offset.abs() > 1e-6 {
        geo = grow_geometry(&geo, total_offset);
    }

    let ops = wrap_vector_outline(Ops::from_geometry(&geo)?, "frame");
    let meta = AssemblyMeta {
        start: ToolPose {
            pos: Point3D::new(0.0, 0.0, 0.0),
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::new(0.0, 0.0, 0.0),
            heading: 0.0,
        },
    };
    Ok((ops, meta))
}

/// Closed outline of a `w × h` rectangle anchored at the origin.
///
/// When `radius` is positive the corners are rounded to that exact
/// radius using circular fillets. The loop starts and ends at the
/// midpoint of the bottom edge so that, treated as an open polyline,
/// every actual corner is an internal vertex and therefore rounded.
fn rectangle_outline(w: f64, h: f64, radius: f64) -> Vec<Point3D> {
    let p = |x: f64, y: f64| Point3D::new(x, y, 0.0);
    if radius <= 1e-9 {
        return vec![p(0.0, 0.0), p(w, 0.0), p(w, h), p(0.0, h), p(0.0, 0.0)];
    }
    let start = p(w / 2.0, 0.0);
    let loop_pts =
        vec![start, p(w, 0.0), p(w, h), p(0.0, h), p(0.0, 0.0), start];
    fillet_polyline_3d(&loop_pts, radius)
}

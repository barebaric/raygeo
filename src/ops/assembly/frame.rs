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
use crate::geo::types::Point3D;
use crate::ops::assembly::contour::compute_total_offset;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{AssembleCtx, Assembler};
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
}

impl Assembler for FrameSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "frame: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let (ops, meta) =
            assemble_frame(ctx.size_mm, self.offset_mm, &self.cut_side)
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
/// optionally offset by offset / cut-side.
///
/// Returns `(Ops, AssemblyMeta)` with zero start/end tool poses (the
/// frame is a closed rectangle with no entry/exit move).
pub fn assemble_frame(
    size_mm: (f64, f64),
    offset_mm: f64,
    cut_side: &str,
) -> RaygeoResult<(Ops, AssemblyMeta)> {
    let (w, h) = size_mm;
    if w <= 0.0 || h <= 0.0 {
        return Err(crate::error::RaygeoError::ContourError(
            "Part has invalid or zero size".to_string(),
        ));
    }

    let total_offset = compute_total_offset(offset_mm, cut_side);

    let mut geo = Geometry::new();
    geo.move_to(0.0, 0.0, 0.0);
    geo.line_to(w, 0.0, 0.0);
    geo.line_to(w, h, 0.0);
    geo.line_to(0.0, h, 0.0);
    geo.close_path();

    if total_offset.abs() > 1e-6 {
        geo = grow_geometry(&geo, total_offset);
    }

    let ops = Ops::from_geometry(&geo)?;
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

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
use crate::geo::algo::fitting::linearize_geometry;
use crate::geo::geometry::Geometry;
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
    pub arc_tolerance: f64,
    pub allow_arcs: bool,
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
            self.arc_tolerance,
            self.allow_arcs,
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
/// clamped to what fits the final (offset) frame, so an oversized
/// value degrades gracefully instead of producing an invalid path.
///
/// Rounded corners are emitted as exact circular arcs. Only when the
/// machine cannot execute arcs (`allow_arcs == false`) are they
/// converted to chords, using `arc_tolerance` as the maximum
/// deviation. Constructing the arcs themselves needs no tolerance.
///
/// Returns `(Ops, AssemblyMeta)` with zero start/end tool poses (the
/// frame is a closed rectangle with no entry/exit move).
pub fn assemble_frame(
    size_mm: (f64, f64),
    offset_mm: f64,
    cut_side: &str,
    corner_radius: f64,
    arc_tolerance: f64,
    allow_arcs: bool,
) -> RaygeoResult<(Ops, AssemblyMeta)> {
    let (w, h) = size_mm;
    if w <= 0.0 || h <= 0.0 {
        return Err(crate::error::RaygeoError::ContourError(
            "Part has invalid or zero size".to_string(),
        ));
    }

    let total_offset = compute_total_offset(offset_mm, cut_side);

    // Apply the offset analytically: a rectangle offset by `d` stays a
    // rectangle anchored at `-d` with each side grown by `2d`. The
    // requested radius applies to this final frame, so rounding
    // happens *after* the offset.
    let origin = -total_offset;
    let final_w = w + 2.0 * total_offset;
    let final_h = h + 2.0 * total_offset;

    let geo = if final_w <= 1e-9 || final_h <= 1e-9 {
        Geometry::new()
    } else {
        rounded_rectangle(origin, origin, final_w, final_h, corner_radius)
    };

    let raw = if allow_arcs || arc_tolerance <= 0.0 {
        Ops::from_geometry(&geo)?
    } else {
        let mut linear = geo.copy();
        linear.data = linearize_geometry(&geo.data, arc_tolerance);
        Ops::from_geometry(&linear)?
    };
    let ops = wrap_vector_outline(raw, "frame");
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

/// Closed `w × h` rectangle with its lower-left corner at `(x, y)`.
///
/// When `radius` is positive each corner is replaced with an exact
/// circular arc of that radius (clamped to half the shorter side),
/// traced counter-clockwise. The path starts and ends on the bottom
/// edge so the outline is already closed.
fn rounded_rectangle(x: f64, y: f64, w: f64, h: f64, radius: f64) -> Geometry {
    let x1 = x + w;
    let y1 = y + h;
    let r = radius.clamp(0.0, w.min(h) / 2.0);

    let mut geo = Geometry::new();
    if r <= 1e-9 {
        geo.move_to(x, y, 0.0);
        geo.line_to(x1, y, 0.0);
        geo.line_to(x1, y1, 0.0);
        geo.line_to(x, y1, 0.0);
        geo.line_to(x, y, 0.0);
        return geo;
    }

    geo.move_to(x + r, y, 0.0);
    geo.line_to(x1 - r, y, 0.0);
    geo.arc_to(x1, y + r, 0.0, r, false, 0.0);
    geo.line_to(x1, y1 - r, 0.0);
    geo.arc_to(x1 - r, y1, -r, 0.0, false, 0.0);
    geo.line_to(x + r, y1, 0.0);
    geo.arc_to(x, y1 - r, 0.0, -r, false, 0.0);
    geo.line_to(x, y + r, 0.0);
    geo.arc_to(x + r, y, r, 0.0, false, 0.0);
    geo
}

//! Shrinkwrap assembler: generate a shrink-wrapped (concave hull)
//! contour around image content.
//!
//! Pure-Rust core. The Python `shrinkwrap` pyfunction in
//! `crate::python::ops::assembly::shrinkwrap` is a thin wrapper that
//! calls [`assemble_shrinkwrap`] and packs the result into a
//! [`PyAssemblyResult`](crate::python::ops::assembly::result::PyAssemblyResult).
//!
//! The [`ShrinkwrapSpec`] struct implements the [`Assembler`] trait so
//! callers can dispatch to it without knowing the concrete parameter
//! set.

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::algo::fitting::{fit_curves, linearize_geometry};
use crate::geo::algo::hull::get_concave_hull;
use crate::geo::algo::offset::grow_geometry;
use crate::geo::geometry::Geometry;
use crate::geo::types::{Point, Point3D};
use crate::ops::assembly::contour::compute_total_offset;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{wrap_vector_outline, AssembleCtx, Assembler};
use crate::ops::container::Ops;
use crate::ops::part::ImageSource;
use crate::ops::types::ToolPose;

/// Spec for the shrinkwrap assembler.
///
/// Mirrors the parameter list of [`assemble_shrinkwrap`]. Held as
/// `Box<dyn Assembler>` by callers that drive the trait.
#[derive(Clone, Debug)]
pub struct ShrinkwrapSpec {
    pub gravity: f64,
    pub offset_mm: f64,
    pub cut_side: String,
    pub arc_tolerance: f64,
    pub allow_arcs: bool,
    pub supports_curves: bool,
}

impl Assembler for ShrinkwrapSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "shrinkwrap: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        if std::env::var("RAYGEO_HULL_TRACE").is_ok() {
            eprintln!(
                "[hull] shrinkwrap face={} size_mm={:?} gravity={}",
                ctx.face_id, ctx.size_mm, self.gravity
            );
        }
        let image_src = ctx.image_source.ok_or_else(|| {
            "Part has no image — set part.image before calling shrinkwrap"
                .to_string()
        })?;
        let (ops, meta) = assemble_shrinkwrap(
            image_src,
            ctx.size_mm,
            self.gravity,
            self.offset_mm,
            &self.cut_side,
            self.arc_tolerance,
            self.allow_arcs,
            self.supports_curves,
        )
        .map_err(|e| e.to_string())?;
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        ctx.trace.append_ops(&ops);
        ctx.callbacks.report_progress(1.0, "shrinkwrap: done");
        Ok(meta)
    }

    fn is_scalable(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "shrinkwrap"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Convert hull points from pixel space to millimetre space.
fn hull_to_mm(
    hull_pts: &[Point],
    part_size_mm: (f64, f64),
    img_shape: (usize, usize),
) -> Geometry {
    let (w_mm, h_mm) = part_size_mm;
    let (h_px, w_px) = img_shape;
    let scale_x = if w_px > 0 { w_mm / w_px as f64 } else { 1.0 };
    let scale_y = if h_px > 0 { h_mm / h_px as f64 } else { 1.0 };

    let mut geo = Geometry::new();
    if !hull_pts.is_empty() {
        let x = hull_pts[0].x * scale_x;
        let y = (h_px as f64 - hull_pts[0].y) * scale_y;
        geo.move_to(x, y, 0.0);
    }
    for p in &hull_pts[1..] {
        let x = p.x * scale_x;
        let y = (h_px as f64 - p.y) * scale_y;
        geo.line_to(x, y, 0.0);
    }
    geo.close_path();
    geo
}

/// Generate a shrink-wrapped (concave hull) contour around image
/// content.
///
/// Reads pixels from `image_src`, computes a concave hull by relaxing
/// a band around the content under gravity, transforms pixel
/// coordinates to millimetre space via `part_size_mm` and the image
/// dimensions, computes the total offset from offset / cut-side,
/// applies it, optionally fits arcs/curves when `arc_tolerance` > 0,
/// and returns the result as an `(Ops, AssemblyMeta)` pair.
#[allow(clippy::too_many_arguments)]
pub fn assemble_shrinkwrap(
    image_src: &dyn ImageSource,
    part_size_mm: (f64, f64),
    gravity: f64,
    offset_mm: f64,
    cut_side: &str,
    arc_tolerance: f64,
    allow_arcs: bool,
    supports_curves: bool,
) -> RaygeoResult<(Ops, AssemblyMeta)> {
    let (w_mm, h_mm) = part_size_mm;
    if w_mm <= 0.0 || h_mm <= 0.0 {
        return Err(RaygeoError::ContourError(
            "Part has invalid or zero size".to_string(),
        ));
    }

    let (w_px, h_px) = image_src.dimensions();
    let raw = image_src.read_all().ok_or_else(|| {
        RaygeoError::ContourError(
            "Part's image source cannot materialise a full buffer — \
             shrinkwrap requires an in-memory image"
                .to_string(),
        )
    })?;
    let (h_px, w_px) = (h_px as usize, w_px as usize);
    let flat: Vec<u8> =
        raw.iter().map(|&v| if v != 0 { 1 } else { 0 }).collect();
    if flat.iter().all(|&v| v == 0) {
        return Ok((
            Ops::new(),
            AssemblyMeta {
                start: ToolPose {
                    pos: Point3D::ZERO,
                    heading: 0.0,
                },
                end: ToolPose {
                    pos: Point3D::ZERO,
                    heading: 0.0,
                },
            },
        ));
    }

    let hull_pts = get_concave_hull(&flat, w_px, h_px, gravity, false)
        .ok_or_else(|| {
            RaygeoError::ContourError(
                "Could not compute concave hull from image".to_string(),
            )
        })?;

    let mut geo = hull_to_mm(&hull_pts, (w_mm, h_mm), (h_px, w_px));

    let total_offset = compute_total_offset(offset_mm, cut_side);
    if total_offset.abs() > 1e-6 {
        geo = grow_geometry(&geo, total_offset);
    }

    let ops = wrap_vector_outline(
        if arc_tolerance > 0.0 {
            let apply_curves = allow_arcs || supports_curves;
            let new_data = if apply_curves {
                fit_curves(
                    &geo.data,
                    arc_tolerance,
                    supports_curves,
                    allow_arcs,
                    None,
                )
            } else {
                linearize_geometry(&geo.data, arc_tolerance)
            };
            let mut fitted = geo.copy();
            fitted.data = new_data;
            Ops::from_geometry(&fitted)?
        } else {
            Ops::from_geometry(&geo)?
        },
        "shrinkwrap",
    );

    let meta = AssemblyMeta {
        start: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
    };
    Ok((ops, meta))
}

//! Contour assembler: trace vector outlines into Ops.
//!
//! Pure-Rust core. The Python `contour` pyfunction in
//! `crate::python::ops::assembly::contour` is a thin wrapper that
//! calls [`assemble_contour`] and packs the result into a
//! [`PyAssemblyResult`](crate::python::ops::assembly::result::PyAssemblyResult).
//!
//! The [`ContourSpec`] struct implements the [`Assembler`] trait so
//! callers can dispatch to it without knowing the concrete parameter
//! set.

use crate::error::RaygeoResult;
use crate::geo::algo::fitting::{fit_curves, linearize_geometry};
use crate::geo::algo::offset::grow_geometry;
use crate::geo::algo::overcut::apply_overcut;
use crate::geo::algo::topology::{
    normalize_winding_orders, remove_inner_edges,
    split_inner_and_outer_contours, split_into_contours,
};
use crate::geo::geometry::Geometry;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{AssembleCtx, Assembler};
use crate::ops::container::Ops;
use crate::ops::part::FaceState;
use crate::ops::types::ToolPose;
use crate::types::Point3D;

/// Spec for the contour assembler.
///
/// Mirrors the parameter list of [`assemble_contour`]. Held as
/// `Box<dyn Assembler>` by callers that drive the trait.
#[derive(Clone, Debug)]
pub struct ContourSpec {
    pub kerf_mm: f64,
    pub path_offset_mm: f64,
    pub cut_side: String,
    pub overcut: f64,
    pub cut_order: String,
    pub remove_inner: bool,
    pub arc_tolerance: f64,
    pub allow_arcs: bool,
    pub supports_curves: bool,
}

impl Assembler for ContourSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "contour: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let (ops, meta) = assemble_contour(
            ctx.face,
            self.kerf_mm,
            self.path_offset_mm,
            &self.cut_side,
            self.overcut,
            &self.cut_order,
            self.remove_inner,
            self.arc_tolerance,
            self.allow_arcs,
            self.supports_curves,
        )
        .map_err(|e| e.to_string())?;
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        ctx.trace.append_ops(&ops);
        ctx.callbacks.report_progress(1.0, "contour: done");
        Ok(meta)
    }

    fn name(&self) -> &'static str {
        "contour"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Compute the total offset from kerf, path offset, and cut side.
///
/// ```text
/// kerf_compensation = kerf_mm / 2
/// centerline  -> 0
/// outside     -> +path_offset_mm + kerf_compensation
/// inside      -> -path_offset_mm - kerf_compensation
/// ```
pub fn compute_total_offset(
    kerf_mm: f64,
    path_offset_mm: f64,
    cut_side: &str,
) -> f64 {
    let kerf_comp = kerf_mm / 2.0;
    match cut_side.to_ascii_lowercase().as_str() {
        "outside" => path_offset_mm + kerf_comp,
        "inside" => -path_offset_mm - kerf_comp,
        _ => 0.0,
    }
}

/// Trace contours from `part.geometry` into an [`Ops`] sequence.
///
/// Extracts the vector geometry from `part`, computes the total offset
/// from kerf / path-offset / cut-side, applies it with winding-order
/// normalisation and offset fallback, orders inner/outer contours,
/// applies overcut, optionally fits arcs and curves, and returns the
/// result as an `(Ops, AssemblyMeta)` pair.
///
/// Returns empty `Ops` if the part has no geometry.
#[allow(clippy::too_many_arguments)]
pub fn assemble_contour(
    face: &FaceState,
    kerf_mm: f64,
    path_offset_mm: f64,
    cut_side: &str,
    overcut: f64,
    cut_order: &str,
    remove_inner: bool,
    arc_tolerance: f64,
    allow_arcs: bool,
    supports_curves: bool,
) -> RaygeoResult<(Ops, AssemblyMeta)> {
    let total_offset = compute_total_offset(kerf_mm, path_offset_mm, cut_side);

    let source_geo = match face.geometry.clone() {
        Some(g) => g,
        None => {
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
            ))
        }
    };

    // 1. Split into contours, separate closed from open.
    let all_contours = split_into_contours(&source_geo);
    let mut closed: Vec<Geometry> = Vec::new();
    let mut open: Vec<Geometry> = Vec::new();
    for c in &all_contours {
        if c.is_closed(1e-6) {
            closed.push(c.copy());
        } else {
            open.push(c.copy());
        }
    }

    // 2. Normalise winding orders so solids are CCW, holes are CW.
    //    This must happen before offset so that grow() shrinks holes
    //    and expands solids in the correct direction.
    let closed_refs: Vec<&Geometry> = closed.iter().collect();
    closed = normalize_winding_orders(&closed_refs);

    // 3. Build composite closed geometry
    let mut composite = Geometry::new();
    for c in &closed {
        composite.extend(c);
    }

    // 4. Apply offset to closed contours (open contours skip offset).
    let offset_applied = if total_offset.abs() > 1e-6 {
        let original_geo = composite.copy();
        let grown_geo = grow_geometry(&composite, total_offset);
        // Fallback: if offset produces empty geometry, keep original.
        if grown_geo.is_empty() && !original_geo.is_empty() {
            original_geo
        } else {
            grown_geo
        }
    } else {
        composite
    };

    // 5. Re-append open contours (they were never offset).
    let mut geo = offset_applied;
    for c in &open {
        geo.extend(c);
    }

    let zero_meta = || AssemblyMeta {
        start: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
    };

    // 6. Handle remove_inner shortcut
    if remove_inner {
        geo = remove_inner_edges(&geo);
        if overcut > 0.0 {
            let contours = split_into_contours(&geo);
            let mut result = Geometry::new();
            for c in &contours {
                result.extend(&apply_overcut(c, overcut));
            }
            geo = result;
        }
        let ops =
            ops_from_geo(&geo, arc_tolerance, allow_arcs, supports_curves)?;
        return Ok((ops, zero_meta()));
    }

    if geo.is_empty() {
        return Ok((Ops::new(), zero_meta()));
    }

    // 7. Re-split after offset, then order inner/outer
    let after = split_into_contours(&geo);
    let mut closed_after: Vec<&Geometry> = Vec::new();
    let mut open_after: Vec<&Geometry> = Vec::new();
    for c in &after {
        if c.is_closed(1e-6) {
            closed_after.push(c);
        } else {
            open_after.push(c);
        }
    }

    let mut ordered = Geometry::new();
    if !closed_after.is_empty() {
        let (inner, outer) = split_inner_and_outer_contours(&closed_after);
        let inner_first = cut_order.eq_ignore_ascii_case("inside_outside");
        let groups: &[&[usize]] = if inner_first {
            &[&inner, &outer]
        } else {
            &[&outer, &inner]
        };

        for group in groups {
            for &idx in *group {
                let mut contour = closed_after[idx].copy();
                if overcut > 0.0 {
                    contour = apply_overcut(&contour, overcut);
                }
                ordered.extend(&contour);
            }
        }
    }
    for c in &open_after {
        ordered.extend(c);
    }

    let ops =
        ops_from_geo(&ordered, arc_tolerance, allow_arcs, supports_curves)?;
    Ok((ops, zero_meta()))
}

/// Convert geometry to Ops, optionally applying curve fitting.
fn ops_from_geo(
    geo: &Geometry,
    arc_tolerance: f64,
    allow_arcs: bool,
    supports_curves: bool,
) -> RaygeoResult<Ops> {
    if arc_tolerance <= 0.0 {
        return Ops::from_geometry(geo);
    }
    let apply_curves = allow_arcs || supports_curves;
    let new_data = if apply_curves {
        fit_curves(&geo.data, arc_tolerance, supports_curves, allow_arcs, None)
    } else {
        linearize_geometry(&geo.data, arc_tolerance)
    };
    let mut fitted = geo.copy();
    fitted.data = new_data;
    Ops::from_geometry(&fitted)
}

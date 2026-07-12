use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::fitting::{fit_curves, linearize_geometry};
use crate::geo::algo::offset::grow_geometry;
use crate::geo::algo::overcut::apply_overcut;
use crate::geo::algo::topology::{
    normalize_winding_orders, remove_inner_edges,
    split_inner_and_outer_contours, split_into_contours,
};
use crate::geo::geometry::Geometry;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::container::Ops;
use crate::ops::types::ToolPose;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::part::part::PyPart;
use crate::types::Point3D;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "contour")?;
    m.add_function(pyo3::wrap_pyfunction!(contour_py, m.clone())?)?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.contour", &m)?;

    Ok(())
}

/// Compute the total offset from kerf, path offset, and cut side.
///
/// Mirrors the logic that used to live in every rayforge producer:
///
/// ```text
/// kerf_compensation = kerf_mm / 2
/// centerline  -> 0
/// outside     -> +path_offset_mm + kerf_compensation
/// inside      -> -path_offset_mm - kerf_compensation
/// ```
pub(crate) fn compute_total_offset(
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

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def contour(
        part: raygeo.ops.part.Part,
        kerf_mm: float = 0.0,
        path_offset_mm: float = 0.0,
        cut_side: str = "centerline",
        overcut: float = 0.0,
        cut_order: str = "inside_outside",
        remove_inner: bool = False,
        arc_tolerance: float = 0.0,
        allow_arcs: bool = True,
        supports_curves: bool = False,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Trace contours from the part geometry.

        Extracts the vector geometry from *part*, computes the total
        offset from kerf / path-offset / cut-side, applies it with
        winding-order normalisation and offset fallback, orders
        inner/outer contours, applies overcut, optionally fits arcs
        and curves, and returns the result as an
        :class:`AssemblyResult`.

        :param part: The part whose geometry defines the contours.
        :param kerf_mm: Tool kerf width in mm (default 0.0).
        :param path_offset_mm: Additional offset distance in mm
            (default 0.0).
        :param cut_side: ``"centerline"``, ``"outside"``, or
            ``"inside"`` (default ``"centerline"``).
        :param overcut: Distance to extend closed contours past their
            start point (mm, default 0.0).
        :param cut_order: ``"inside_outside"`` or ``"outside_inside"``
            (default ``"inside_outside"``).
        :param remove_inner: Remove inner (hole) contours (default False).
        :param arc_tolerance: Curve fitting tolerance in mm; when > 0
            arcs/beziers are fitted (default 0.0).
        :param allow_arcs: Fit arcs when arc_tolerance > 0 (default True).
        :param supports_curves: Keep Bézier curves when arc_tolerance > 0
            (default False).
        :returns: An :class:`AssemblyResult` with the contour path.
        :raises ValueError: If the part has no geometry.
        """
    "#,
    module = "raygeo.ops.assembly.contour"
)]
#[allow(clippy::too_many_arguments)]
#[pyfunction(name = "contour")]
#[pyo3(signature = (
    part,
    kerf_mm = 0.0,
    path_offset_mm = 0.0,
    cut_side = "centerline",
    overcut = 0.0,
    cut_order = "inside_outside",
    remove_inner = false,
    arc_tolerance = 0.0,
    allow_arcs = true,
    supports_curves = false,
))]
fn contour_py(
    part: &PyPart,
    kerf_mm: f64,
    path_offset_mm: f64,
    cut_side: &str,
    overcut: f64,
    cut_order: &str,
    remove_inner: bool,
    arc_tolerance: f64,
    allow_arcs: bool,
    supports_curves: bool,
) -> PyResult<PyAssemblyResult> {
    let total_offset = compute_total_offset(kerf_mm, path_offset_mm, cut_side);

    let source_geo = part.inner.geometry.clone().ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err("Part has no geometry")
    })?;

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
        return Ok(PyAssemblyResult::from_parts(
            ops_from_geo(&geo, arc_tolerance, allow_arcs, supports_curves)?,
            meta,
            None,
            vec![],
        ));
    }

    if geo.is_empty() {
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
        return Ok(PyAssemblyResult::from_parts(
            Ops::new(),
            meta,
            None,
            vec![],
        ));
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
    let ops =
        ops_from_geo(&ordered, arc_tolerance, allow_arcs, supports_curves)?;
    Ok(PyAssemblyResult::from_parts(ops, meta, None, vec![]))
}

/// Convert geometry to Ops, optionally applying curve fitting.
fn ops_from_geo(
    geo: &Geometry,
    arc_tolerance: f64,
    allow_arcs: bool,
    supports_curves: bool,
) -> PyResult<Ops> {
    if arc_tolerance <= 0.0 {
        return Ok(Ops::from_geometry(geo)?);
    }
    let apply_curves = allow_arcs || supports_curves;
    let new_data = if apply_curves {
        fit_curves(&geo.data, arc_tolerance, supports_curves, allow_arcs, None)
    } else {
        linearize_geometry(&geo.data, arc_tolerance)
    };
    let mut fitted = geo.copy();
    fitted.data = new_data;
    Ok(Ops::from_geometry(&fitted)?)
}

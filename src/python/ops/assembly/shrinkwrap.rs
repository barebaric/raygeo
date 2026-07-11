use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::fitting::{fit_curves, linearize_geometry};
use crate::geo::algo::hull::get_concave_hull;
use crate::geo::algo::offset::grow_geometry;
use crate::geo::geometry::Geometry;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::container::Ops;
use crate::ops::cut::ToolPose;
use crate::python::ops::assembly::contour::compute_total_offset;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::cut::part::PyPart;
use crate::types::Point3D;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "shrinkwrap")?;
    m.add_function(pyo3::wrap_pyfunction!(shrinkwrap_py, m.clone())?)?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.shrinkwrap", &m)?;

    Ok(())
}

fn extract_bool_image(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<(Vec<u8>, usize, usize)> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (obj,))?;
    let shape: (usize, usize) = arr.getattr("shape")?.extract()?;
    let flat: Vec<u8> = arr
        .call_method("astype", ("uint8",), None)?
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    let nonzero: Vec<u8> =
        flat.iter().map(|&v| if v != 0 { 1 } else { 0 }).collect();
    Ok((nonzero, shape.0, shape.1))
}

fn hull_to_mm(
    hull_pts: &[crate::types::Point],
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

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import raygeo

    def shrinkwrap(
        part: raygeo.ops.cut.Part,
        image: numpy.ndarray,
        gravity: float = 0.1,
        kerf_mm: float = 0.0,
        path_offset_mm: float = 0.0,
        cut_side: str = "centerline",
        arc_tolerance: float = 0.0,
        allow_arcs: bool = True,
        supports_curves: bool = False,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Generate a shrink-wrapped (concave hull) contour around image content.

        Computes a concave hull from the binary *image* using Bézier
        gravity attraction, transforms pixel coordinates to millimetre
        space via the part's *size_mm* and image dimensions, computes
        the total offset from kerf / path-offset / cut-side, applies
        it, optionally fits arcs/curves when *arc_tolerance* > 0, and
        returns the result as an :class:`AssemblyResult`.

        :param part: Part providing physical size metadata.
        :param image: 2D boolean or binary numpy array.
        :param gravity: Shrink-wrap factor 0.0–1.0 (0 = convex hull,
            default 0.1).
        :param kerf_mm: Tool kerf width in mm (default 0.0).
        :param path_offset_mm: Additional offset distance in mm
            (default 0.0).
        :param cut_side: ``"centerline"``, ``"outside"``, or
            ``"inside"`` (default ``"centerline"``).
        :param arc_tolerance: Curve fitting tolerance in mm (default 0.0).
        :param allow_arcs: Fit arcs when arc_tolerance > 0 (default True).
        :param supports_curves: Keep Bézier curves when arc_tolerance > 0
            (default False).
        :returns: An :class:`AssemblyResult` with the shrinkwrap path.
        :raises ValueError: If the image is empty or the part has no size.
        """
    "#,
    module = "raygeo.ops.assembly.shrinkwrap"
)]
#[allow(clippy::too_many_arguments)]
#[pyfunction(name = "shrinkwrap")]
#[pyo3(signature = (
    part,
    image,
    gravity = 0.1,
    kerf_mm = 0.0,
    path_offset_mm = 0.0,
    cut_side = "centerline",
    arc_tolerance = 0.0,
    allow_arcs = true,
    supports_curves = false,
))]
fn shrinkwrap_py(
    py: Python<'_>,
    part: &PyPart,
    image: &Bound<'_, PyAny>,
    gravity: f64,
    kerf_mm: f64,
    path_offset_mm: f64,
    cut_side: &str,
    arc_tolerance: f64,
    allow_arcs: bool,
    supports_curves: bool,
) -> PyResult<PyAssemblyResult> {
    let (w_mm, h_mm) = part.inner.size_mm;
    if w_mm <= 0.0 || h_mm <= 0.0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Part has invalid or zero size",
        ));
    }

    let (flat, h_px, w_px) = extract_bool_image(py, image)?;
    if flat.iter().all(|&v| v == 0) {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Image is empty (all background)",
        ));
    }

    let hull_pts =
        get_concave_hull(&flat, w_px, h_px, gravity).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(
                "Could not compute concave hull from image",
            )
        })?;

    let mut geo = hull_to_mm(&hull_pts, (w_mm, h_mm), (h_px, w_px));

    let total_offset = compute_total_offset(kerf_mm, path_offset_mm, cut_side);
    if total_offset.abs() > 1e-6 {
        geo = grow_geometry(&geo, total_offset);
    }

    // Optionally apply curve fitting
    let ops = if arc_tolerance > 0.0 {
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
    };
    Ok(PyAssemblyResult::from_parts(
        ops,
        AssemblyMeta {
            cleared_polygons: vec![],
            start: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
            end: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
        },
        None,
        vec![],
    ))
}

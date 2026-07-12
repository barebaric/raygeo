use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::offset::grow_geometry;
use crate::geo::geometry::Geometry;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::container::Ops;
use crate::ops::types::ToolPose;
use crate::python::ops::assembly::contour::compute_total_offset;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::part::part::PyPart;
use crate::types::Point3D;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "frame")?;
    m.add_function(pyo3::wrap_pyfunction!(frame_py, m.clone())?)?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.frame", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def frame(
        part: raygeo.ops.part.Part,
        kerf_mm: float = 0.0,
        path_offset_mm: float = 0.0,
        cut_side: str = "centerline",
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate a rectangular frame around the part boundary.

        Creates a rectangle matching ``part.size_mm``, computes the
        total offset from kerf / path-offset / cut-side, applies it,
        and returns the frame as an :class:`AssemblyResult`.

        :param part: The part whose size defines the frame.
        :param kerf_mm: Tool kerf width in mm (default 0.0).
        :param path_offset_mm: Additional offset distance in mm
            (default 0.0).
        :param cut_side: ``"centerline"``, ``"outside"``, or
            ``"inside"`` (default ``"centerline"``).
        :returns: An :class:`AssemblyResult` with the frame path.
        :raises ValueError: If the part has no size information.
        """
    "#,
    module = "raygeo.ops.assembly.frame"
)]
#[pyfunction(name = "frame")]
#[pyo3(signature = (
    part,
    kerf_mm = 0.0,
    path_offset_mm = 0.0,
    cut_side = "centerline",
))]
fn frame_py(
    part: &PyPart,
    kerf_mm: f64,
    path_offset_mm: f64,
    cut_side: &str,
) -> PyResult<PyAssemblyResult> {
    let (w, h) = part.inner.size_mm;
    if w <= 0.0 || h <= 0.0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Part has invalid or zero size",
        ));
    }

    let total_offset = compute_total_offset(kerf_mm, path_offset_mm, cut_side);

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
    let start = ToolPose {
        pos: Point3D::new(0.0, 0.0, 0.0),
        heading: 0.0,
    };
    let end = ToolPose {
        pos: Point3D::new(0.0, 0.0, 0.0),
        heading: 0.0,
    };

    Ok(PyAssemblyResult::from_parts(
        ops,
        AssemblyMeta { start, end },
        None,
        vec![],
    ))
}

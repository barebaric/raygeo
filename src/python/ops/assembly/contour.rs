use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::assembly::contour::assemble_contour;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "contour")?;
    m.add_function(pyo3::wrap_pyfunction!(contour_py, m.clone())?)?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.contour", &m)?;

    Ok(())
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
    ) -> raygeo.ops.assembly.AssemblyResult:
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
    let (ops, meta) = assemble_contour(
        &part.inner,
        kerf_mm,
        path_offset_mm,
        cut_side,
        overcut,
        cut_order,
        remove_inner,
        arc_tolerance,
        allow_arcs,
        supports_curves,
    )?;
    Ok(PyAssemblyResult::from_parts(ops, meta, None, vec![]))
}

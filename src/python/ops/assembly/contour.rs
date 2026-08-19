use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::assembly::contour::{
    assemble_contour, ContourSpec as CoreContourSpec,
};
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "contour")?;
    m.add_function(pyo3::wrap_pyfunction!(contour_py, m.clone())?)?;
    m.add_class::<PyContourSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.contour", &m)?;

    Ok(())
}

/// Parameters for the ``contour`` assembler.
///
/// Construct with ``ContourSpec(offset_mm, cut_side,
/// overcut, cut_order, remove_inner, arc_tolerance, allow_arcs,
/// supports_curves)``. Wrap in an
/// :class:`~raygeo.ops.assembly.Assembler` instance to drive the
/// `Assembler` trait.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.contour",
    name = "ContourSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyContourSpec {
    /// Total path offset distance in mm.
    #[pyo3(get)]
    pub offset_mm: f64,
    /// ``"centerline"``, ``"outside"``, or ``"inside"``.
    #[pyo3(get)]
    pub cut_side: String,
    /// Distance to extend closed contours past their start point (mm).
    #[pyo3(get)]
    pub overcut: f64,
    /// ``"inside_outside"`` or ``"outside_inside"``.
    #[pyo3(get)]
    pub cut_order: String,
    /// Remove inner (hole) contours.
    #[pyo3(get)]
    pub remove_inner: bool,
    /// Curve fitting tolerance in mm; when > 0 arcs/beziers are fitted.
    #[pyo3(get)]
    pub arc_tolerance: f64,
    /// Fit arcs when arc_tolerance > 0.
    #[pyo3(get)]
    pub allow_arcs: bool,
    /// Keep Bézier curves when arc_tolerance > 0.
    #[pyo3(get)]
    pub supports_curves: bool,
}

impl PyContourSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreContourSpec {
        CoreContourSpec {
            offset_mm: self.offset_mm,
            cut_side: self.cut_side,
            overcut: self.overcut,
            cut_order: self.cut_order,
            remove_inner: self.remove_inner,
            arc_tolerance: self.arc_tolerance,
            allow_arcs: self.allow_arcs,
            supports_curves: self.supports_curves,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyContourSpec {
    #[new]
    #[pyo3(signature = (
        offset_mm = 0.0,
        cut_side = "centerline",
        overcut = 0.0,
        cut_order = "inside_outside",
        remove_inner = false,
        arc_tolerance = 0.0,
        allow_arcs = true,
        supports_curves = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        offset_mm: f64,
        cut_side: &str,
        overcut: f64,
        cut_order: &str,
        remove_inner: bool,
        arc_tolerance: f64,
        allow_arcs: bool,
        supports_curves: bool,
    ) -> Self {
        PyContourSpec {
            offset_mm,
            cut_side: cut_side.to_string(),
            overcut,
            cut_order: cut_order.to_string(),
            remove_inner,
            arc_tolerance,
            allow_arcs,
            supports_curves,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def contour(
        part: raygeo.ops.part.Part,
        offset_mm: float = 0.0,
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
        offset from offset / cut-side, applies it with
        winding-order normalisation and offset fallback, orders
        inner/outer contours, applies overcut, optionally fits arcs
        and curves, and returns the result as an
        :class:`AssemblyResult`.

        :param part: The part whose geometry defines the contours.
        :param offset_mm: Total path offset distance in mm
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
    offset_mm = 0.0,
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
    offset_mm: f64,
    cut_side: &str,
    overcut: f64,
    cut_order: &str,
    remove_inner: bool,
    arc_tolerance: f64,
    allow_arcs: bool,
    supports_curves: bool,
) -> PyResult<PyAssemblyResult> {
    let face = part.inner.face("").ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err("no default face")
    })?;
    let (ops, meta, _cut_polygons) = assemble_contour(
        face,
        offset_mm,
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

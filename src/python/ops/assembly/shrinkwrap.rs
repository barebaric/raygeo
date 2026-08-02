use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::assembly::shrinkwrap::{
    assemble_shrinkwrap, ShrinkwrapSpec as CoreShrinkwrapSpec,
};
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "shrinkwrap")?;
    m.add_function(pyo3::wrap_pyfunction!(shrinkwrap_py, m.clone())?)?;
    m.add_class::<PyShrinkwrapSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.shrinkwrap", &m)?;

    Ok(())
}

/// Parameters for the ``shrinkwrap`` assembler.
///
/// Construct with ``ShrinkwrapSpec(gravity, offset_mm, ...)``. Wrap in
/// an :class:`~raygeo.ops.assembly.Assembler` instance to drive the
/// `Assembler` trait.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.shrinkwrap",
    name = "ShrinkwrapSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyShrinkwrapSpec {
    #[pyo3(get)]
    pub gravity: f64,
    #[pyo3(get)]
    pub offset_mm: f64,
    #[pyo3(get)]
    pub cut_side: String,
    #[pyo3(get)]
    pub arc_tolerance: f64,
    #[pyo3(get)]
    pub allow_arcs: bool,
    #[pyo3(get)]
    pub supports_curves: bool,
}

impl PyShrinkwrapSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreShrinkwrapSpec {
        CoreShrinkwrapSpec {
            gravity: self.gravity,
            offset_mm: self.offset_mm,
            cut_side: self.cut_side,
            arc_tolerance: self.arc_tolerance,
            allow_arcs: self.allow_arcs,
            supports_curves: self.supports_curves,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyShrinkwrapSpec {
    #[new]
    #[pyo3(signature = (
        gravity = 0.1,
        offset_mm = 0.0,
        cut_side = "centerline",
        arc_tolerance = 0.0,
        allow_arcs = true,
        supports_curves = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        gravity: f64,
        offset_mm: f64,
        cut_side: &str,
        arc_tolerance: f64,
        allow_arcs: bool,
        supports_curves: bool,
    ) -> Self {
        PyShrinkwrapSpec {
            gravity,
            offset_mm,
            cut_side: cut_side.to_string(),
            arc_tolerance,
            allow_arcs,
            supports_curves,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import raygeo

    def shrinkwrap(
        part: raygeo.ops.part.Part,
        gravity: float = 0.1,
        offset_mm: float = 0.0,
        cut_side: str = "centerline",
        arc_tolerance: float = 0.0,
        allow_arcs: bool = True,
        supports_curves: bool = False,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate a shrink-wrapped (concave hull) contour around image content.

        Reads the pixel image from ``part.image`` (a 2-D uint8 numpy
        array), computes a concave hull using Bézier gravity attraction,
        transforms pixel coordinates to millimetre space via the part's
        *size_mm* and image dimensions, computes the total offset from
        offset / cut-side, applies it, optionally fits
        arcs/curves when *arc_tolerance* > 0, and returns the result
        as an :class:`AssemblyResult`.

        :param part: Part providing physical size metadata and the
            image buffer (``part.image``).
        :param gravity: Shrink-wrap factor 0.0–1.0 (0 = convex hull,
            default 0.1).
        :param offset_mm: Total path offset distance in mm
            (default 0.0).
        :param cut_side: ``"centerline"``, ``"outside"``, or
            ``"inside"`` (default ``"centerline"``).
        :param arc_tolerance: Curve fitting tolerance in mm (default 0.0).
        :param allow_arcs: Fit arcs when arc_tolerance > 0 (default True).
        :param supports_curves: Keep Bézier curves when arc_tolerance > 0
            (default False).
        :returns: An :class:`AssemblyResult` with the shrinkwrap path.
        :raises ValueError: If the image is empty, part has no size,
            or ``part.image`` is None.
        """
    "#,
    module = "raygeo.ops.assembly.shrinkwrap"
)]
#[allow(clippy::too_many_arguments)]
#[pyfunction(name = "shrinkwrap")]
#[pyo3(signature = (
    part,
    gravity = 0.1,
    offset_mm = 0.0,
    cut_side = "centerline",
    arc_tolerance = 0.0,
    allow_arcs = true,
    supports_curves = false,
))]
fn shrinkwrap_py(
    part: &PyPart,
    gravity: f64,
    offset_mm: f64,
    cut_side: &str,
    arc_tolerance: f64,
    allow_arcs: bool,
    supports_curves: bool,
) -> PyResult<PyAssemblyResult> {
    let image_src = part.inner.image_source.as_ref().ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(
            "Part has no image — set part.image before calling shrinkwrap",
        )
    })?;
    let (ops, meta) = assemble_shrinkwrap(
        image_src.as_ref(),
        part.inner.size_mm,
        gravity,
        offset_mm,
        cut_side,
        arc_tolerance,
        allow_arcs,
        supports_curves,
    )?;
    Ok(PyAssemblyResult::from_parts(ops, meta, None, vec![]))
}

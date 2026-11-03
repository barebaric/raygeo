use crate::ops::assembly::erosion::{self, ErosionSpec};
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "erosion")?;
    m.add_function(pyo3::wrap_pyfunction!(generate_erosion_py, m.clone())?)?;
    m.add_class::<PyErosionSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.erosion", &m)?;

    Ok(())
}

/// Parameters for the ``erosion`` assembler.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.erosion",
    name = "ErosionSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyErosionSpec {
    #[pyo3(get)]
    pub tool_radius: f64,
    #[pyo3(get)]
    pub target_z: f64,
    /// Travel height for the retract between carriers.
    #[pyo3(get)]
    pub safe_z: f64,
    /// Leave the pocket alone when the actionable residual totals less
    /// than this area (mm²).
    #[pyo3(get)]
    pub area_tolerance: f64,
}

impl PyErosionSpec {
    pub fn into_core(self) -> ErosionSpec {
        ErosionSpec {
            tool_radius: self.tool_radius,
            target_z: self.target_z,
            safe_z: self.safe_z,
            area_tolerance: self.area_tolerance,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyErosionSpec {
    #[new]
    #[pyo3(signature = (
        tool_radius = 3.0,
        target_z = -5.0,
        safe_z = 2.0,
        area_tolerance = 1.0,
    ))]
    fn new(
        tool_radius: f64,
        target_z: f64,
        safe_z: f64,
        area_tolerance: f64,
    ) -> Self {
        PyErosionSpec {
            tool_radius,
            target_z,
            safe_z,
            area_tolerance,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def generate_erosion(
        part: raygeo.ops.part.Part,
        tool_radius: float = 3.0,
        target_z: float = -5.0,
        safe_z: float = 2.0,
        area_tolerance: float = 1.0,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Erode the residual clumps the clearing pass left behind.

        Finds the uncut material inside the tool-centre envelope (a
        phantom-scale inset drops the sweep-approximation film), slots
        each clump along a spine carrier at *target_z*, and records the
        cut footprint in the cleared area. Emits no ops when the
        residual totals less than *area_tolerance*.

        :param part: The part whose ``cleared`` field tracks accumulated
                     workpiece state and whose geometry defines the
                     pocket boundary and islands.
        :param tool_radius: Tool radius in mm (default 3.0).
        :param target_z: Cutting Z height (default -5.0).
        :param safe_z: Travel height for the retract between carriers
                       (default 2.0).
        :param area_tolerance: Skip when the residual is below this
                               area in mm² (default 1.0).
        :param state: Optional machine state to apply before the path.
        :returns: An :class:`AssemblyResult` with the erosion path
                  (empty when there is nothing to erode).
        """
    "#,
    module = "raygeo.ops.assembly.erosion"
)]
#[pyfunction(name = "generate_erosion")]
#[pyo3(signature = (
    part,
    tool_radius = 3.0,
    target_z = -5.0,
    safe_z = 2.0,
    area_tolerance = 1.0,
    state = None,
))]
fn generate_erosion_py(
    part: &mut crate::python::ops::part::part::PyPart,
    tool_radius: f64,
    target_z: f64,
    safe_z: f64,
    area_tolerance: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let cut_state = match state {
        Some(ref s) => s.borrow().0.clone(),
        None => State::default(),
    };

    let opts = ErosionSpec {
        tool_radius,
        target_z,
        safe_z,
        area_tolerance,
    };

    let mut trace = Tracelet::new();
    let face = part.inner.face_mut("");
    let meta = erosion::generate_erosion(face, &mut trace, &opts, &cut_state)?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}

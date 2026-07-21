use crate::ops::assembly::slot::{self, SlotSpec};
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "slot")?;
    m.add_function(pyo3::wrap_pyfunction!(generate_slot_py, m.clone())?)?;
    m.add_class::<PySlotSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.slot", &m)?;

    Ok(())
}

/// Parameters for the ``slot`` assembler.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.slot",
    name = "SlotSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PySlotSpec {
    /// ``(x, y)`` carrier waypoints.
    #[pyo3(get)]
    pub carrier: Vec<(f64, f64)>,
    #[pyo3(get)]
    pub tool_radius: f64,
    #[pyo3(get)]
    pub target_z: f64,
}

impl PySlotSpec {
    pub fn into_core(self) -> SlotSpec {
        SlotSpec {
            carrier: self
                .carrier
                .into_iter()
                .map(|(x, y)| Point::new(x, y))
                .collect(),
            tool_radius: self.tool_radius,
            target_z: self.target_z,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PySlotSpec {
    #[new]
    fn new(carrier: Vec<(f64, f64)>, tool_radius: f64, target_z: f64) -> Self {
        PySlotSpec {
            carrier,
            tool_radius,
            target_z,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def generate_slot(
        part: raygeo.ops.part.Part,
        carrier: collections.abc.Sequence[tuple[float, float]],
        tool_radius: float,
        target_z: float,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate a back-and-forth slot clearing path along a carrier.

        Produces a forward pass then a backward pass along the carrier,
        both at constant *target_z*. The cleared polygon is the carrier
        swept by *tool_radius* (Minkowski sum).

        :param carrier: ``(x, y)`` waypoints (currently 2-point segment).
        :param tool_radius: Tool radius in mm.
        :param target_z: Cutting Z height.
        :param state: Optional machine state to apply before the path.
        :returns: An :class:`AssemblyResult` with the slot path.
        """
    "#,
    module = "raygeo.ops.assembly.slot"
)]
#[pyfunction(name = "generate_slot")]
#[pyo3(signature = (part, carrier, tool_radius, target_z, state = None))]
fn generate_slot_py(
    part: &mut crate::python::ops::part::part::PyPart,
    carrier: Vec<(f64, f64)>,
    tool_radius: f64,
    target_z: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let cut_state = match state {
        Some(ref s) => s.borrow().0.clone(),
        None => State::default(),
    };

    let carrier_pts: Vec<Point> =
        carrier.into_iter().map(|(x, y)| Point::new(x, y)).collect();

    let opts = SlotSpec {
        carrier: carrier_pts,
        tool_radius,
        target_z,
    };

    let mut trace = Tracelet::new();
    let face = part.inner.face_mut("");
    let meta = slot::generate_slot(face, &mut trace, &opts, &cut_state)?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}

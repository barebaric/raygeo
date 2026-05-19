use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use raygeo_core::ops::axis::Axis;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAxis>()?;
    Ok(())
}

#[gen_stub_pyclass]
#[pyclass(frozen, skip_from_py_object, module = "raygeo.ops", name = "Axis")]
#[derive(Clone)]
pub struct PyAxis(pub Axis);

#[gen_stub_pymethods]
#[pymethods]
impl PyAxis {
    #[classattr]
    pub const X: PyAxis = PyAxis(Axis::X);
    #[classattr]
    pub const Y: PyAxis = PyAxis(Axis::Y);
    #[classattr]
    pub const Z: PyAxis = PyAxis(Axis::Z);
    #[classattr]
    pub const A: PyAxis = PyAxis(Axis::A);
    #[classattr]
    pub const B: PyAxis = PyAxis(Axis::B);
    #[classattr]
    pub const C: PyAxis = PyAxis(Axis::C);
    #[classattr]
    pub const U: PyAxis = PyAxis(Axis::U);

    fn __or__(&self, other: &Self) -> Self {
        PyAxis(self.0 | other.0)
    }

    fn __and__(&self, other: &Self) -> Self {
        PyAxis(self.0 & other.0)
    }

    fn __xor__(&self, other: &Self) -> Self {
        PyAxis(self.0 ^ other.0)
    }

    fn __invert__(&self) -> Self {
        PyAxis(!self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __ne__(&self, other: &Self) -> bool {
        self.0 != other.0
    }

    fn __repr__(&self) -> String {
        format!("Axis.{}", self.label())
    }

    fn __hash__(&self) -> u64 {
        self.0.bits() as u64
    }

    #[getter]
    fn value(&self) -> u8 {
        self.0.bits()
    }

    #[getter]
    fn label(&self) -> String {
        match self.0.label() {
            Ok(s) => s.to_uppercase(),
            Err(_) => format!("{:?}", self.0),
        }
    }

    fn assert_single_axis(&self) -> PyResult<()> {
        self.0
            .assert_single_axis()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

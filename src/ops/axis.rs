use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use raygeo_core::ops::axis::Axis;

pub(crate) const MODULE_DOC: &str = "\
Axis bitflag for multi-axis machines.

Represents a single axis or combination of axes (X, Y, Z, A, B, C, U).
Axis values can be combined using bitwise operators (|, &, ^, ~) to
represent multiple axes at once, useful when specifying which axes
participate in a coordinated move or transformation.
";

/// Register the Axis enum with the Python module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__doc__", MODULE_DOC)?;
    m.add_class::<PyAxis>()?;
    Ok(())
}

/// Represents a single axis or a combination of axes (X, Y, Z, A, B, C, U).
///
/// Axis values can be combined using bitwise operators (``|``, ``&``, ``^``, ``~``)
/// to represent multiple axes at once.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "raygeo.ops.axis",
    name = "Axis"
)]
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

    /// Combine two axis masks with bitwise OR.
    fn __or__(&self, other: &Self) -> Self {
        PyAxis(self.0 | other.0)
    }

    /// Intersect two axis masks with bitwise AND.
    fn __and__(&self, other: &Self) -> Self {
        PyAxis(self.0 & other.0)
    }

    /// Compute the symmetric difference of two axis masks.
    fn __xor__(&self, other: &Self) -> Self {
        PyAxis(self.0 ^ other.0)
    }

    /// Invert (complement) the axis mask.
    fn __invert__(&self) -> Self {
        PyAxis(!self.0)
    }

    /// Check equality of two Axes.
    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    /// Check inequality of two Axes.
    fn __ne__(&self, other: &Self) -> bool {
        self.0 != other.0
    }

    /// Return a string representation like ``Axis.X``.
    fn __repr__(&self) -> String {
        format!("Axis.{}", self.label())
    }

    /// Hash based on the raw bit value.
    fn __hash__(&self) -> u64 {
        self.0.bits() as u64
    }

    /// The raw bit value of the axis.
    #[getter]
    fn value(&self) -> u8 {
        self.0.bits()
    }

    /// The uppercase label of the axis (e.g. ``"X"``, ``"Y"``, ``"Z"``).
    #[getter]
    fn label(&self) -> String {
        match self.0.label() {
            Ok(s) => s.to_uppercase(),
            Err(_) => format!("{:?}", self.0),
        }
    }

    /// Assert that this Axis represents exactly one axis (not a combination).
    ///
    /// :raises ValueError: If the axis mask contains multiple or zero bits set.
    fn assert_single_axis(&self) -> PyResult<()> {
        self.0
            .assert_single_axis()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

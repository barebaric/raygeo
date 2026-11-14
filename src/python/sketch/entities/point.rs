//! Bindings for the sketch point.

use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// A sketch point: coordinates and solver flags.
///
/// When obtained from a registry it is a live view: coordinate writes
/// are written back to the registry. It also behaves like an ``(x, y)``
/// pair for indexing and iteration.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch.entities.point", name = "SketchPoint")]
pub struct PySketchPoint {
    pub inner: crate::sketch::entities::point::SketchPoint,
    /// Live back-reference to the owning registry, if any.
    pub registry: Option<Py<PyAny>>,
}

impl PySketchPoint {
    /// Builds a live point view bound to a registry.
    pub fn live(
        inner: crate::sketch::entities::point::SketchPoint,
        registry: &Bound<'_, PyAny>,
    ) -> Self {
        PySketchPoint {
            inner,
            registry: Some(registry.clone().unbind()),
        }
    }

    fn write_back(&self) {
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                let _ = registry.bind(py).call_method1(
                    "set_point_position",
                    (self.inner.id, self.inner.x, self.inner.y),
                );
            });
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySketchPoint {
    /// Creates a point view with the given id and coordinates.
    #[new]
    #[pyo3(signature = (id, x, y, fixed = false))]
    fn new(id: u64, x: f64, y: f64, fixed: bool) -> Self {
        PySketchPoint {
            inner: crate::sketch::entities::point::SketchPoint {
                id,
                x,
                y,
                fixed,
                waypoint_type:
                    crate::sketch::entities::point::WaypointType::Sharp,
                constrained: false,
            },
            registry: None,
        }
    }

    /// Unique point id, stable across solves.
    #[getter]
    fn id(&self) -> u64 {
        self.inner.id
    }

    /// X coordinate.
    #[getter]
    fn x(&self) -> f64 {
        self.inner.x
    }

    #[setter]
    fn set_x(&mut self, value: f64) {
        self.inner.x = value;
        self.write_back();
    }

    /// Y coordinate.
    #[getter]
    fn y(&self) -> f64 {
        self.inner.y
    }

    #[setter]
    fn set_y(&mut self, value: f64) {
        self.inner.y = value;
        self.write_back();
    }

    /// Whether the point is pinned and never moves.
    #[getter]
    fn fixed(&self) -> bool {
        self.inner.fixed
    }

    #[setter]
    fn set_fixed(&mut self, value: bool) {
        self.inner.fixed = value;
    }

    /// Whether the point is fully constrained, as computed by the
    /// solver.
    #[getter]
    fn constrained(&self) -> bool {
        self.inner.constrained
    }

    #[setter]
    fn set_constrained(&mut self, value: bool) {
        self.inner.constrained = value;
    }

    /// Index 0 gives x, index 1 gives y (pair-like access).
    fn __getitem__(&self, index: i64) -> PyResult<f64> {
        match index {
            0 | -2 => Ok(self.inner.x),
            1 | -1 => Ok(self.inner.y),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "point index out of range",
            )),
        }
    }

    /// Writes a coordinate and persists it to the owning registry.
    fn __setitem__(&mut self, index: i64, value: f64) {
        match index {
            0 | -2 => self.inner.x = value,
            1 | -1 => self.inner.y = value,
            _ => {}
        }
        self.write_back();
    }

    /// The point is a pair of coordinates.
    fn __len__(&self) -> usize {
        2
    }

    /// Iterates ``(x, y)``.
    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let pair = PyTuple::new(py, [self.inner.x, self.inner.y])?;
        pair.call_method0("__iter__")
    }

    /// Compares equal to an ``(x, y)`` pair or another point with the
    /// same coordinates.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(p) = other.extract::<PyRef<'_, PySketchPoint>>() {
            return self.inner.x == p.inner.x && self.inner.y == p.inner.y;
        }
        if let Ok((x, y)) = other.extract::<(f64, f64)>() {
            return self.inner.x == x && self.inner.y == y;
        }
        if let (Ok(x), Ok(y)) = (other.get_item(0), other.get_item(1)) {
            if let (Ok(x), Ok(y)) = (x.extract::<f64>(), y.extract::<f64>()) {
                return self.inner.x == x && self.inner.y == y;
            }
        }
        false
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.x.to_bits().hash(&mut hasher);
        self.inner.y.to_bits().hash(&mut hasher);
        hasher.finish()
    }

    /// Returns a readable representation.
    fn __repr__(&self) -> String {
        format!(
            "SketchPoint(id={}, x={}, y={}, fixed={})",
            self.inner.id, self.inner.x, self.inner.y, self.inner.fixed
        )
    }
}

/// Registers the point class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySketchPoint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities.point", m)?;
    Ok(())
}

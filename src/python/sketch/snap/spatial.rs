//! Sorted snap line index.

use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::types::PySnapLine;

#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.sketch.snap.spatial", name = "IndexedLine")]
#[derive(Clone)]
pub struct PyIndexedLine {
    pub snap_line: Option<Py<PySnapLine>>,
    pub coordinate: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyIndexedLine {
    #[new]
    #[pyo3(signature = (snap_line=None, coordinate=0.0))]
    fn new(snap_line: Option<Py<PySnapLine>>, coordinate: f64) -> Self {
        Self {
            snap_line,
            coordinate,
        }
    }

    #[getter]
    fn snap_line(&self, py: Python<'_>) -> Option<Py<PySnapLine>> {
        self.snap_line.as_ref().map(|v| v.clone_ref(py))
    }

    #[getter]
    fn coordinate(&self) -> f64 {
        self.coordinate
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.coordinate < other.coordinate
    }
}

#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.sketch.snap.spatial", name = "SnapLineIndex")]
#[derive(Default, Clone)]
pub struct PySnapLineIndex {
    pub horizontal: Vec<PyIndexedLine>,
    pub vertical: Vec<PyIndexedLine>,
    pub dirty: bool,
}

impl PySnapLineIndex {
    pub(crate) fn add_line(
        &mut self,
        py: Python<'_>,
        snap_line: Py<PySnapLine>,
    ) {
        let borrowed = snap_line.borrow(py);
        let is_horizontal = borrowed.is_horizontal;
        let coordinate = borrowed.coordinate;
        drop(borrowed);
        let entry = PyIndexedLine {
            snap_line: Some(snap_line),
            coordinate,
        };
        if is_horizontal {
            Self::insert(&mut self.horizontal, entry);
        } else {
            Self::insert(&mut self.vertical, entry);
        }
        self.dirty = true;
    }

    fn insert(
        entries: &mut Vec<PyIndexedLine>,
        entry: PyIndexedLine,
    ) {
        let pos = entries
            .iter()
            .position(|item| item.coordinate > entry.coordinate)
            .unwrap_or(entries.len());
        entries.insert(pos, entry);
    }

    fn query_entries(
        entries: &[PyIndexedLine],
        value: f64,
        threshold: f64,
        py: Python<'_>,
    ) -> Vec<(Py<PySnapLine>, f64)> {
        let mut results = Vec::new();
        for indexed in entries {
            if let Some(line) = &indexed.snap_line {
                let dist = (value - indexed.coordinate).abs();
                if dist <= threshold {
                    results.push((line.clone_ref(py), dist));
                }
            }
        }
        results
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySnapLineIndex {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn clear(&mut self) {
        self.horizontal.clear();
        self.vertical.clear();
        self.dirty = false;
    }

    fn add(&mut self, py: Python<'_>, snap_line: &Bound<'_, PySnapLine>) {
        self.add_line(py, snap_line.clone().unbind());
    }

    fn add_all(
        &mut self,
        py: Python<'_>,
        snap_lines: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        for item in snap_lines.try_iter()? {
            let item = item?;
            if item.is_none() {
                return Err(PyAttributeError::new_err(
                    "'NoneType' object has no attribute 'is_horizontal'",
                ));
            }
            self.add_line(py, item.extract::<Py<PySnapLine>>()?);
        }
        Ok(())
    }

    fn query_horizontal(
        &self,
        y: f64,
        threshold: f64,
        py: Python<'_>,
    ) -> Vec<(Py<PySnapLine>, f64)> {
        Self::query_entries(&self.horizontal, y, threshold, py)
    }

    fn query_vertical(
        &self,
        x: f64,
        threshold: f64,
        py: Python<'_>,
    ) -> Vec<(Py<PySnapLine>, f64)> {
        Self::query_entries(&self.vertical, x, threshold, py)
    }

    fn query(
        &self,
        x: f64,
        y: f64,
        threshold: f64,
        py: Python<'_>,
    ) -> Vec<(Py<PySnapLine>, f64)> {
        let mut results = self.query_horizontal(y, threshold, py);
        results.extend(self.query_vertical(x, threshold, py));
        results.sort_by(|a, b| {
            let left = a.1;
            let right = b.1;
            left.partial_cmp(&right)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let ap = a.0.borrow(py).line_type.priority_value();
                    let bp = b.0.borrow(py).line_type.priority_value();
                    bp.cmp(&ap)
                })
        });
        results
    }

    #[getter]
    fn _horizontal(&self, py: Python<'_>) -> Vec<Py<PyIndexedLine>> {
        self.horizontal
            .iter()
            .map(|v| Py::new(py, v.clone()).unwrap())
            .collect()
    }

    #[getter]
    fn _vertical(&self, py: Python<'_>) -> Vec<Py<PyIndexedLine>> {
        self.vertical
            .iter()
            .map(|v| Py::new(py, v.clone()).unwrap())
            .collect()
    }

    #[getter]
    fn _dirty(&self) -> bool {
        self.dirty
    }

    fn __len__(&self) -> usize {
        self.horizontal.len() + self.vertical.len()
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyIndexedLine>()?;
    module.add_class::<PySnapLineIndex>()?;
    Ok(())
}

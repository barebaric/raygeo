use pyo3::prelude::*;

use raygeo_core::ops::{OpsSection, OpsSectionRange};

use super::container::PyOps;
use super::enums::PySectionType;

#[pyclass(module = "raygeo.ops", name = "OpsSection", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOpsSection(pub OpsSection);

#[pymethods]
impl PyOpsSection {
    #[getter]
    fn section_type(&self) -> Option<PySectionType> {
        self.0.section_type.map(PySectionType)
    }

    #[getter]
    fn marker_indices(&self) -> Vec<usize> {
        self.0.marker_indices.clone()
    }

    #[getter]
    fn content_indices(&self) -> Vec<usize> {
        self.0.content_indices.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "OpsSection(section_type={:?}, marker_indices={:?}, content_indices={:?})",
            self.0.section_type, self.0.marker_indices, self.0.content_indices
        )
    }
}

#[pyclass(module = "raygeo.ops", name = "OpsSectionRange", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOpsSectionRange(pub OpsSectionRange);

#[pymethods]
impl PyOpsSectionRange {
    #[getter]
    fn section_type(&self) -> Option<PySectionType> {
        self.0.section_type.map(PySectionType)
    }

    #[getter]
    fn marker_indices(&self) -> Vec<usize> {
        self.0.marker_indices.clone()
    }

    #[getter]
    fn content_indices(&self) -> Vec<usize> {
        self.0.content_indices.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "OpsSectionRange(section_type={:?}, marker_indices={:?}, content_indices={:?})",
            self.0.section_type, self.0.marker_indices, self.0.content_indices
        )
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "group")?;
    m.add_function(wrap_pyfunction!(py_segments, &m)?)?;
    m.add_function(wrap_pyfunction!(py_segment_indices, &m)?)?;
    m.add_function(wrap_pyfunction!(py_without_state, &m)?)?;
    m.add_function(wrap_pyfunction!(py_group_by_state_continuity, &m)?)?;
    m.add_function(wrap_pyfunction!(py_split_into_subpaths, &m)?)?;
    m.add_function(wrap_pyfunction!(py_iter_sections, &m)?)?;
    m.add_function(wrap_pyfunction!(py_iter_section_ranges, &m)?)?;
    m.add_class::<PyOpsSection>()?;
    m.add_class::<PyOpsSectionRange>()?;
    parent.add_submodule(&m)?;

    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.group", &m)?;

    Ok(())
}

#[pyfunction]
fn py_segments(ops: &PyOps) -> Vec<Vec<usize>> {
    ops.inner.segments()
}

#[pyfunction]
fn py_segment_indices(ops: &PyOps) -> Vec<Vec<usize>> {
    ops.inner.segment_indices()
}

#[pyfunction]
fn py_without_state(ops: &PyOps) -> PyOps {
    PyOps {
        inner: ops.inner.without_state(),
    }
}

#[pyfunction]
fn py_group_by_state_continuity(ops: &PyOps) -> Vec<PyOps> {
    ops.inner
        .group_by_state_continuity()
        .into_iter()
        .map(|o| PyOps { inner: o })
        .collect()
}

#[pyfunction]
fn py_split_into_subpaths(ops: &PyOps) -> Vec<PyOps> {
    ops.inner
        .split_into_subpaths()
        .into_iter()
        .map(|o| PyOps { inner: o })
        .collect()
}

#[pyfunction]
fn py_iter_sections(ops: &PyOps) -> Vec<PyOpsSection> {
    ops.inner
        .iter_sections()
        .into_iter()
        .map(PyOpsSection)
        .collect()
}

#[pyfunction]
fn py_iter_section_ranges(ops: &PyOps) -> Vec<PyOpsSectionRange> {
    ops.inner
        .iter_section_ranges()
        .into_iter()
        .map(PyOpsSectionRange)
        .collect()
}

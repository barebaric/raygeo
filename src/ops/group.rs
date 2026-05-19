use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods, gen_stub_pyfunction};
use raygeo_core::ops::{OpsSection, OpsSectionRange};

use super::container::PyOps;
use super::enums::PySectionType;

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops", name = "OpsSection", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOpsSection(pub OpsSection);

#[gen_stub_pymethods]
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

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops", name = "OpsSectionRange", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOpsSectionRange(pub OpsSectionRange);

#[gen_stub_pymethods]
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

#[gen_stub_pyfunction(python = r#"
    def segments(ops: Ops) -> list[list[int]]:
        """Return the segment indices of the ops."""
"#, module = "raygeo.ops")]
#[pyfunction]
fn py_segments(ops: &PyOps) -> Vec<Vec<usize>> {
    ops.inner.segments()
}

#[gen_stub_pyfunction(python = r#"
    def segment_indices(ops: Ops) -> list[list[int]]:
        """Return the segment indices of the ops."""
"#, module = "raygeo.ops")]
#[pyfunction]
fn py_segment_indices(ops: &PyOps) -> Vec<Vec<usize>> {
    ops.inner.segment_indices()
}

#[gen_stub_pyfunction(python = r#"
    def without_state(ops: Ops) -> Ops:
        """Return a copy of the ops without state commands."""
"#, module = "raygeo.ops")]
#[pyfunction]
fn py_without_state(ops: &PyOps) -> PyOps {
    PyOps {
        inner: ops.inner.without_state(),
    }
}

#[gen_stub_pyfunction(python = r#"
    def group_by_state_continuity(ops: Ops) -> list[Ops]:
        """Group ops by state continuity."""
"#, module = "raygeo.ops")]
#[pyfunction]
fn py_group_by_state_continuity(ops: &PyOps) -> Vec<PyOps> {
    ops.inner
        .group_by_state_continuity()
        .into_iter()
        .map(|o| PyOps { inner: o })
        .collect()
}

#[gen_stub_pyfunction(python = r#"
    def split_into_subpaths(ops: Ops) -> list[Ops]:
        """Split ops into subpaths."""
"#, module = "raygeo.ops")]
#[pyfunction]
fn py_split_into_subpaths(ops: &PyOps) -> Vec<PyOps> {
    ops.inner
        .split_into_subpaths()
        .into_iter()
        .map(|o| PyOps { inner: o })
        .collect()
}

#[gen_stub_pyfunction(python = r#"
    def iter_sections(ops: Ops) -> list[OpsSection]:
        """Iterate over the sections of the ops."""
"#, module = "raygeo.ops")]
#[pyfunction]
fn py_iter_sections(ops: &PyOps) -> Vec<PyOpsSection> {
    ops.inner
        .iter_sections()
        .into_iter()
        .map(PyOpsSection)
        .collect()
}

#[gen_stub_pyfunction(python = r#"
    def iter_section_ranges(ops: Ops) -> list[OpsSectionRange]:
        """Iterate over the section ranges of the ops."""
"#, module = "raygeo.ops")]
#[pyfunction]
fn py_iter_section_ranges(ops: &PyOps) -> Vec<PyOpsSectionRange> {
    ops.inner
        .iter_section_ranges()
        .into_iter()
        .map(PyOpsSectionRange)
        .collect()
}

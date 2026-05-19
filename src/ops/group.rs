use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods, gen_stub_pyfunction};
use raygeo_core::ops::{OpsSection, OpsSectionRange};

use super::container::PyOps;
use super::enums::PySectionType;

/// A section of operations parsed into marker and content index groups.
///
/// Produced by :func:`iter_sections` when splitting an Ops sequence
/// into logical sections based on ``OpsSectionStart``/``OpsSectionEnd`` markers.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.group", name = "OpsSection", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOpsSection(pub OpsSection);

#[gen_stub_pymethods]
#[pymethods]
impl PyOpsSection {
    /// The type of this section (VectorOutline or RasterFill), if any.
    #[getter]
    fn section_type(&self) -> Option<PySectionType> {
        self.0.section_type.map(PySectionType)
    }

    /// Indices of the section-marker commands (start/end) for this section.
    #[getter]
    fn marker_indices(&self) -> Vec<usize> {
        self.0.marker_indices.clone()
    }

    /// Indices of the content commands belonging to this section.
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

/// A contiguous range of indices that belong to a section.
///
/// Similar to :class:`OpsSection` but stores start/end index ranges
/// instead of individual index lists. Produced by :func:`iter_section_ranges`.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.group", name = "OpsSectionRange", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOpsSectionRange(pub OpsSectionRange);

#[gen_stub_pymethods]
#[pymethods]
impl PyOpsSectionRange {
    /// The type of this section range (VectorOutline or RasterFill), if any.
    #[getter]
    fn section_type(&self) -> Option<PySectionType> {
        self.0.section_type.map(PySectionType)
    }

    /// Indices of the section-marker commands that bracket this range.
    #[getter]
    fn marker_indices(&self) -> Vec<usize> {
        self.0.marker_indices.clone()
    }

    /// Starting index of the content within this section range.
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

/// Register the ``raygeo.ops.group`` submodule with the parent module.
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
        """Return groups of command indices that form contiguous path segments.

        Each segment is a list of command indices connected end-to-end
        (separated by travel moves).

        :param ops: The operations to segment.
        :returns: List of index groups, one per segment.
        """
"#, module = "raygeo.ops.group")]
#[pyfunction]
fn py_segments(ops: &PyOps) -> Vec<Vec<usize>> {
    ops.inner.segments()
}

#[gen_stub_pyfunction(python = r#"
    def segment_indices(ops: Ops) -> list[list[int]]:
        """Return the segment start/end index pairs of the ops.

        Each element is a ``[start, end]`` pair representing a contiguous
        range of command indices that form a segment.

        :param ops: The operations to analyze.
        :returns: List of ``[start, end]`` index pairs.
        """
"#, module = "raygeo.ops.group")]
#[pyfunction]
fn py_segment_indices(ops: &PyOps) -> Vec<Vec<usize>> {
    ops.inner.segment_indices()
}

#[gen_stub_pyfunction(python = r#"
    def without_state(ops: Ops) -> Ops:
        """Return a copy of the ops with all state commands removed.

        State commands (SetPower, SetCutSpeed, etc.) are stripped,
        leaving only moving and marker commands.

        :param ops: The operations to filter.
        :returns: A new Ops with only non-state commands.
        """
"#, module = "raygeo.ops.group")]
#[pyfunction]
fn py_without_state(ops: &PyOps) -> PyOps {
    PyOps {
        inner: ops.inner.without_state(),
    }
}

#[gen_stub_pyfunction(python = r#"
    def group_by_state_continuity(ops: Ops) -> list[Ops]:
        """Split ops into groups where the machine state does not change.

        Whenever a state-changing command (power, speed, etc.) is
        encountered, a new group begins. This is useful for determining
        where the machine settings are consistent.

        :param ops: The operations to split.
        :returns: List of Ops groups, each with uniform state.
        """
"#, module = "raygeo.ops.group")]
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
        """Split ops into individual subpaths (separated by MoveTo commands).

        Each MoveTo command starts a new subpath.

        :param ops: The operations to split.
        :returns: List of Ops, one per subpath.
        """
"#, module = "raygeo.ops.group")]
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
        """Iterate over the logical sections of the ops.

        Sections are delimited by ``OpsSectionStart``/``OpsSectionEnd``
        markers and group commands into vector-outline and raster-fill
        portions.

        :param ops: The operations to inspect.
        :returns: List of OpsSection objects.
        """
"#, module = "raygeo.ops.group")]
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
        """Iterate over the section ranges of the ops as index ranges.

        Similar to :func:`iter_sections` but returns contiguous
        index ranges instead of individual index lists.

        :param ops: The operations to inspect.
        :returns: List of OpsSectionRange objects.
        """
"#, module = "raygeo.ops.group")]
#[pyfunction]
fn py_iter_section_ranges(ops: &PyOps) -> Vec<PyOpsSectionRange> {
    ops.inner
        .iter_section_ranges()
        .into_iter()
        .map(PyOpsSectionRange)
        .collect()
}

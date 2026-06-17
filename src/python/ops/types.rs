use crate::ops::enums::{CommandCategory, CommandType, SectionType};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

pyo3_stub_gen::module_doc!("raygeo.ops.types", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Core enumerations for the Ops command system.

CommandType — identifies each command in a sequence (MoveTo, LineTo,
ArcTo, BezierTo, SetPower, SetFeedRate, JobStart, etc.).

CommandCategory — classifies commands as MOVING (changes tool position),
STATE (changes machine parameters), or MARKER (structural job boundaries).

SectionType — distinguishes between VECTOR_OUTLINE and RASTER_FILL sections
when an Ops sequence is split into logical portions.
";

/// Register the CommandType, CommandCategory, SectionType enums
/// and the category() function with the Python module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__doc__", MODULE_DOC)?;
    m.add_class::<PyCommandType>()?;
    m.add_class::<PyCommandCategory>()?;
    m.add_class::<PySectionType>()?;
    m.add_function(wrap_pyfunction!(py_category, m)?)?;
    Ok(())
}

/// Enumeration of all command types in an Ops sequence.
///
/// Each constant represents a specific operation command such as
/// ``MOVE_TO``, ``LINE_TO``, ``ARC_TO``, ``SET_POWER``, etc.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    skip_from_py_object,
    module = "raygeo.ops.types",
    name = "CommandType"
)]
#[derive(Clone, PartialEq)]
pub struct PyCommandType(pub CommandType);

#[gen_stub_pymethods]
#[pymethods]
impl PyCommandType {
    #[classattr]
    pub const MOVE_TO: PyCommandType = PyCommandType(CommandType::MoveTo);
    #[classattr]
    pub const LINE_TO: PyCommandType = PyCommandType(CommandType::LineTo);
    #[classattr]
    pub const ARC_TO: PyCommandType = PyCommandType(CommandType::ArcTo);
    #[classattr]
    pub const SCAN_LINE: PyCommandType = PyCommandType(CommandType::ScanLine);
    #[classattr]
    pub const DWELL: PyCommandType = PyCommandType(CommandType::Dwell);
    #[classattr]
    pub const BEZIER_TO: PyCommandType = PyCommandType(CommandType::BezierTo);
    #[classattr]
    pub const QUADRATIC_BEZIER_TO: PyCommandType =
        PyCommandType(CommandType::QuadraticBezierTo);
    #[classattr]
    pub const SET_POWER: PyCommandType = PyCommandType(CommandType::SetPower);
    #[classattr]
    pub const SET_FEED_RATE: PyCommandType =
        PyCommandType(CommandType::SetFeedRate);
    #[classattr]
    pub const SET_RAPID_RATE: PyCommandType =
        PyCommandType(CommandType::SetRapidRate);
    #[classattr]
    pub const SET_HEAD: PyCommandType = PyCommandType(CommandType::SetHead);
    #[classattr]
    pub const SET_FREQUENCY: PyCommandType =
        PyCommandType(CommandType::SetFrequency);
    #[classattr]
    pub const SET_PULSE_WIDTH: PyCommandType =
        PyCommandType(CommandType::SetPulseWidth);
    #[classattr]
    pub const SET_SPINDLE_RPM: PyCommandType =
        PyCommandType(CommandType::SetSpindleRpm);
    #[classattr]
    pub const SET_COOLANT: PyCommandType =
        PyCommandType(CommandType::SetCoolant);
    #[classattr]
    pub const JOB_START: PyCommandType = PyCommandType(CommandType::JobStart);
    #[classattr]
    pub const JOB_END: PyCommandType = PyCommandType(CommandType::JobEnd);
    #[classattr]
    pub const LAYER_START: PyCommandType =
        PyCommandType(CommandType::LayerStart);
    #[classattr]
    pub const LAYER_END: PyCommandType = PyCommandType(CommandType::LayerEnd);
    #[classattr]
    pub const WORKPIECE_START: PyCommandType =
        PyCommandType(CommandType::WorkpieceStart);
    #[classattr]
    pub const WORKPIECE_END: PyCommandType =
        PyCommandType(CommandType::WorkpieceEnd);
    #[classattr]
    pub const OPS_SECTION_START: PyCommandType =
        PyCommandType(CommandType::OpsSectionStart);
    #[classattr]
    pub const OPS_SECTION_END: PyCommandType =
        PyCommandType(CommandType::OpsSectionEnd);

    /// String representation like ``CommandType.MOVE_TO``.
    ///
    /// :complexity: O(1)
    fn __repr__(&self) -> String {
        format!("CommandType.{}", self.name())
    }

    /// The raw integer value of this command type.
    ///
    /// :complexity: O(1)
    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }

    /// The uppercase name of this command type (e.g. ``"MOVE_TO"``, ``"LINE_TO"``).
    ///
    /// :complexity: O(1)
    #[getter]
    fn name(&self) -> String {
        self.0.to_string()
    }
}

/// Represents the category of a command: ``MOVING``, ``STATE``, or ``MARKER``.
///
/// - **MOVING**: Commands that change the tool position (MoveTo, LineTo, ArcTo, etc.)
/// - **STATE**: Commands that change machine state (SetPower, SetCutSpeed, etc.)
/// - **MARKER**: Structural markers (JobStart/End, LayerStart/End, etc.)
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    skip_from_py_object,
    module = "raygeo.ops.types",
    name = "CommandCategory"
)]
#[derive(Clone, PartialEq)]
pub struct PyCommandCategory(pub CommandCategory);

#[gen_stub_pymethods]
#[pymethods]
impl PyCommandCategory {
    #[classattr]
    pub const MOVING: PyCommandCategory =
        PyCommandCategory(CommandCategory::Moving);
    #[classattr]
    pub const STATE: PyCommandCategory =
        PyCommandCategory(CommandCategory::State);
    #[classattr]
    pub const MARKER: PyCommandCategory =
        PyCommandCategory(CommandCategory::Marker);

    /// String representation like ``CommandCategory.MOVING``.
    ///
    /// :complexity: O(1)
    fn __repr__(&self) -> String {
        format!("CommandCategory.{}", self.name())
    }

    /// The raw integer value of this category.
    ///
    /// :complexity: O(1)
    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }

    /// The uppercase name of this category (``"MOVING"``, ``"STATE"``, or ``"MARKER"``).
    ///
    /// :complexity: O(1)
    #[getter]
    fn name(&self) -> String {
        match self.0 {
            CommandCategory::Moving => "MOVING",
            CommandCategory::State => "STATE",
            CommandCategory::Marker => "MARKER",
        }
        .to_string()
    }
}

/// The type of an operations section: ``VECTOR_OUTLINE`` or ``RASTER_FILL``.
///
/// Sections divide an Ops sequence into vector and raster portions.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    skip_from_py_object,
    module = "raygeo.ops.types",
    name = "SectionType"
)]
#[derive(Clone, PartialEq)]
pub struct PySectionType(pub SectionType);

#[gen_stub_pymethods]
#[pymethods]
impl PySectionType {
    #[classattr]
    pub const VECTOR_OUTLINE: PySectionType =
        PySectionType(SectionType::VectorOutline);
    #[classattr]
    pub const RASTER_FILL: PySectionType =
        PySectionType(SectionType::RasterFill);

    /// String representation like ``SectionType.VECTOR_OUTLINE``.
    ///
    /// :complexity: O(1)
    fn __repr__(&self) -> String {
        format!("SectionType.{}", self.name())
    }

    /// The raw integer value of this section type.
    ///
    /// :complexity: O(1)
    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }

    /// The uppercase name (``"VECTOR_OUTLINE"`` or ``"RASTER_FILL"``).
    ///
    /// :complexity: O(1)
    #[getter]
    fn name(&self) -> String {
        self.0.to_string()
    }
}

#[gen_stub_pyfunction(
    python = r#"
    def category(ct: CommandType) -> CommandCategory:
        """Get the category of a command type.

        :complexity: O(1)
        """
    "#,
    module = "raygeo.ops.types"
)]
#[pyfunction(name = "category")]
fn py_category(ct: &PyCommandType) -> PyCommandCategory {
    PyCommandCategory(ct.0.category())
}

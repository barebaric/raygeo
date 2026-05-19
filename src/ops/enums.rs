use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods, gen_stub_pyfunction};
use raygeo_core::ops::enums::{category, CommandCategory, CommandType, SectionType};

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCommandType>()?;
    m.add_class::<PyCommandCategory>()?;
    m.add_class::<PySectionType>()?;
    m.add_function(wrap_pyfunction!(py_category, m)?)?;
    Ok(())
}

#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object, module = "raygeo.ops", name = "CommandType")]
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
    pub const QUADRATIC_BEZIER_TO: PyCommandType = PyCommandType(CommandType::QuadraticBezierTo);
    #[classattr]
    pub const SET_POWER: PyCommandType = PyCommandType(CommandType::SetPower);
    #[classattr]
    pub const SET_CUT_SPEED: PyCommandType = PyCommandType(CommandType::SetCutSpeed);
    #[classattr]
    pub const SET_TRAVEL_SPEED: PyCommandType = PyCommandType(CommandType::SetTravelSpeed);
    #[classattr]
    pub const ENABLE_AIR_ASSIST: PyCommandType = PyCommandType(CommandType::EnableAirAssist);
    #[classattr]
    pub const DISABLE_AIR_ASSIST: PyCommandType = PyCommandType(CommandType::DisableAirAssist);
    #[classattr]
    pub const SET_LASER: PyCommandType = PyCommandType(CommandType::SetLaser);
    #[classattr]
    pub const SET_FREQUENCY: PyCommandType = PyCommandType(CommandType::SetFrequency);
    #[classattr]
    pub const SET_PULSE_WIDTH: PyCommandType = PyCommandType(CommandType::SetPulseWidth);
    #[classattr]
    pub const JOB_START: PyCommandType = PyCommandType(CommandType::JobStart);
    #[classattr]
    pub const JOB_END: PyCommandType = PyCommandType(CommandType::JobEnd);
    #[classattr]
    pub const LAYER_START: PyCommandType = PyCommandType(CommandType::LayerStart);
    #[classattr]
    pub const LAYER_END: PyCommandType = PyCommandType(CommandType::LayerEnd);
    #[classattr]
    pub const WORKPIECE_START: PyCommandType = PyCommandType(CommandType::WorkpieceStart);
    #[classattr]
    pub const WORKPIECE_END: PyCommandType = PyCommandType(CommandType::WorkpieceEnd);
    #[classattr]
    pub const OPS_SECTION_START: PyCommandType = PyCommandType(CommandType::OpsSectionStart);
    #[classattr]
    pub const OPS_SECTION_END: PyCommandType = PyCommandType(CommandType::OpsSectionEnd);

    fn __repr__(&self) -> String {
        format!("CommandType.{}", self.name())
    }

    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }

    #[getter]
    fn name(&self) -> String {
        match self.0 {
            CommandType::MoveTo => "MOVE_TO",
            CommandType::LineTo => "LINE_TO",
            CommandType::ArcTo => "ARC_TO",
            CommandType::ScanLine => "SCAN_LINE",
            CommandType::Dwell => "DWELL",
            CommandType::BezierTo => "BEZIER_TO",
            CommandType::QuadraticBezierTo => "QUADRATIC_BEZIER_TO",
            CommandType::SetPower => "SET_POWER",
            CommandType::SetCutSpeed => "SET_CUT_SPEED",
            CommandType::SetTravelSpeed => "SET_TRAVEL_SPEED",
            CommandType::EnableAirAssist => "ENABLE_AIR_ASSIST",
            CommandType::DisableAirAssist => "DISABLE_AIR_ASSIST",
            CommandType::SetLaser => "SET_LASER",
            CommandType::SetFrequency => "SET_FREQUENCY",
            CommandType::SetPulseWidth => "SET_PULSE_WIDTH",
            CommandType::JobStart => "JOB_START",
            CommandType::JobEnd => "JOB_END",
            CommandType::LayerStart => "LAYER_START",
            CommandType::LayerEnd => "LAYER_END",
            CommandType::WorkpieceStart => "WORKPIECE_START",
            CommandType::WorkpieceEnd => "WORKPIECE_END",
            CommandType::OpsSectionStart => "OPS_SECTION_START",
            CommandType::OpsSectionEnd => "OPS_SECTION_END",
        }.to_string()
    }
}

#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object, module = "raygeo.ops", name = "CommandCategory")]
#[derive(Clone, PartialEq)]
pub struct PyCommandCategory(pub CommandCategory);

#[gen_stub_pymethods]
#[pymethods]
impl PyCommandCategory {
    #[classattr]
    pub const MOVING: PyCommandCategory = PyCommandCategory(CommandCategory::Moving);
    #[classattr]
    pub const STATE: PyCommandCategory = PyCommandCategory(CommandCategory::State);
    #[classattr]
    pub const MARKER: PyCommandCategory = PyCommandCategory(CommandCategory::Marker);

    fn __repr__(&self) -> String {
        format!("CommandCategory.{}", self.name())
    }

    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }

    #[getter]
    fn name(&self) -> String {
        match self.0 {
            CommandCategory::Moving => "MOVING",
            CommandCategory::State => "STATE",
            CommandCategory::Marker => "MARKER",
        }.to_string()
    }
}

#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object, module = "raygeo.ops", name = "SectionType")]
#[derive(Clone, PartialEq)]
pub struct PySectionType(pub SectionType);

#[gen_stub_pymethods]
#[pymethods]
impl PySectionType {
    #[classattr]
    pub const VECTOR_OUTLINE: PySectionType = PySectionType(SectionType::VectorOutline);
    #[classattr]
    pub const RASTER_FILL: PySectionType = PySectionType(SectionType::RasterFill);

    fn __repr__(&self) -> String {
        format!("SectionType.{}", self.name())
    }

    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }

    #[getter]
    fn name(&self) -> String {
        match self.0 {
            SectionType::VectorOutline => "VECTOR_OUTLINE",
            SectionType::RasterFill => "RASTER_FILL",
        }.to_string()
    }
}

#[gen_stub_pyfunction(python = r#"
    def category(ct: CommandType) -> CommandCategory:
        """Get the category of a command type."""
"#, module = "raygeo.ops")]
#[pyfunction(name = "category")]
fn py_category(ct: &PyCommandType) -> PyCommandCategory {
    PyCommandCategory(category(ct.0))
}

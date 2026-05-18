use pyo3::prelude::*;
use raygeo_core::ops::enums::{category, CommandCategory, CommandType, SectionType};

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCommandType>()?;
    m.add_class::<PyCommandCategory>()?;
    m.add_class::<PySectionType>()?;
    m.add_function(wrap_pyfunction!(py_category, m)?)?;
    Ok(())
}

#[pyclass(frozen, eq, skip_from_py_object, name = "CommandType")]
#[derive(Clone, PartialEq)]
pub struct PyCommandType(pub CommandType);

#[pymethods]
impl PyCommandType {
    #[classattr]
    pub const MOVE_TO: Self = PyCommandType(CommandType::MoveTo);
    #[classattr]
    pub const LINE_TO: Self = PyCommandType(CommandType::LineTo);
    #[classattr]
    pub const ARC_TO: Self = PyCommandType(CommandType::ArcTo);
    #[classattr]
    pub const SCAN_LINE: Self = PyCommandType(CommandType::ScanLine);
    #[classattr]
    pub const DWELL: Self = PyCommandType(CommandType::Dwell);
    #[classattr]
    pub const BEZIER_TO: Self = PyCommandType(CommandType::BezierTo);
    #[classattr]
    pub const QUADRATIC_BEZIER_TO: Self = PyCommandType(CommandType::QuadraticBezierTo);
    #[classattr]
    pub const SET_POWER: Self = PyCommandType(CommandType::SetPower);
    #[classattr]
    pub const SET_CUT_SPEED: Self = PyCommandType(CommandType::SetCutSpeed);
    #[classattr]
    pub const SET_TRAVEL_SPEED: Self = PyCommandType(CommandType::SetTravelSpeed);
    #[classattr]
    pub const ENABLE_AIR_ASSIST: Self = PyCommandType(CommandType::EnableAirAssist);
    #[classattr]
    pub const DISABLE_AIR_ASSIST: Self = PyCommandType(CommandType::DisableAirAssist);
    #[classattr]
    pub const SET_LASER: Self = PyCommandType(CommandType::SetLaser);
    #[classattr]
    pub const SET_FREQUENCY: Self = PyCommandType(CommandType::SetFrequency);
    #[classattr]
    pub const SET_PULSE_WIDTH: Self = PyCommandType(CommandType::SetPulseWidth);
    #[classattr]
    pub const JOB_START: Self = PyCommandType(CommandType::JobStart);
    #[classattr]
    pub const JOB_END: Self = PyCommandType(CommandType::JobEnd);
    #[classattr]
    pub const LAYER_START: Self = PyCommandType(CommandType::LayerStart);
    #[classattr]
    pub const LAYER_END: Self = PyCommandType(CommandType::LayerEnd);
    #[classattr]
    pub const WORKPIECE_START: Self = PyCommandType(CommandType::WorkpieceStart);
    #[classattr]
    pub const WORKPIECE_END: Self = PyCommandType(CommandType::WorkpieceEnd);
    #[classattr]
    pub const OPS_SECTION_START: Self = PyCommandType(CommandType::OpsSectionStart);
    #[classattr]
    pub const OPS_SECTION_END: Self = PyCommandType(CommandType::OpsSectionEnd);

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

#[pyclass(frozen, eq, skip_from_py_object, name = "CommandCategory")]
#[derive(Clone, PartialEq)]
pub struct PyCommandCategory(pub CommandCategory);

#[pymethods]
impl PyCommandCategory {
    #[classattr]
    pub const MOVING: Self = PyCommandCategory(CommandCategory::Moving);
    #[classattr]
    pub const STATE: Self = PyCommandCategory(CommandCategory::State);
    #[classattr]
    pub const MARKER: Self = PyCommandCategory(CommandCategory::Marker);

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

#[pyclass(frozen, eq, skip_from_py_object, name = "SectionType")]
#[derive(Clone, PartialEq)]
pub struct PySectionType(pub SectionType);

#[pymethods]
impl PySectionType {
    #[classattr]
    pub const VECTOR_OUTLINE: Self = PySectionType(SectionType::VectorOutline);
    #[classattr]
    pub const RASTER_FILL: Self = PySectionType(SectionType::RasterFill);

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

#[pyfunction]
fn py_category(ct: &PyCommandType) -> PyCommandCategory {
    PyCommandCategory(category(ct.0))
}

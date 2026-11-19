//! Python bindings for sketch document data.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods,
};

use crate::python::sketch::registry::{value_from_dict, value_to_py};
use crate::python::sketch::types::EntityId;
use crate::sketch::document::{
    Fill as CoreFill, FillStyle as CoreFillStyle, DEFAULT_FILL_COLOR,
    FORMAT_VERSION,
};

/// Fill rendering style.
#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.sketch.document",
    name = "FillStyle",
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyFillStyle {
    #[pyo3(name = "SOLID")]
    Solid,
    #[pyo3(name = "LINEAR_GRADIENT")]
    LinearGradient,
    #[pyo3(name = "RADIAL_GRADIENT")]
    RadialGradient,
}

impl From<PyFillStyle> for CoreFillStyle {
    fn from(style: PyFillStyle) -> Self {
        match style {
            PyFillStyle::Solid => CoreFillStyle::Solid,
            PyFillStyle::LinearGradient => CoreFillStyle::LinearGradient,
            PyFillStyle::RadialGradient => CoreFillStyle::RadialGradient,
        }
    }
}

impl From<CoreFillStyle> for PyFillStyle {
    fn from(style: CoreFillStyle) -> Self {
        match style {
            CoreFillStyle::Solid => PyFillStyle::Solid,
            CoreFillStyle::LinearGradient => PyFillStyle::LinearGradient,
            CoreFillStyle::RadialGradient => PyFillStyle::RadialGradient,
        }
    }
}

#[pymethods]
impl PyFillStyle {
    fn __repr__(&self) -> String {
        match self {
            PyFillStyle::Solid => "FillStyle.SOLID".to_string(),
            PyFillStyle::LinearGradient => {
                "FillStyle.LINEAR_GRADIENT".to_string()
            }
            PyFillStyle::RadialGradient => {
                "FillStyle.RADIAL_GRADIENT".to_string()
            }
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    #[getter]
    fn name(&self) -> &str {
        match self {
            PyFillStyle::Solid => "SOLID",
            PyFillStyle::LinearGradient => "LINEAR_GRADIENT",
            PyFillStyle::RadialGradient => "RADIAL_GRADIENT",
        }
    }

    #[getter]
    fn value(&self) -> &str {
        match self {
            PyFillStyle::Solid => "solid",
            PyFillStyle::LinearGradient => "linear_gradient",
            PyFillStyle::RadialGradient => "radial_gradient",
        }
    }
}

type Color4 = (f64, f64, f64, f64);

fn color_to_array(color: Color4) -> [f64; 4] {
    [color.0, color.1, color.2, color.3]
}

fn color_from_array(color: [f64; 4]) -> Color4 {
    (color[0], color[1], color[2], color[3])
}

/// A filled area bounded by sketch entities.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.document",
    name = "Fill",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFill {
    inner: CoreFill,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyFill {
    #[new]
    #[pyo3(signature = (
        uid,
        boundary,
        style = PyFillStyle::Solid,
        color = (DEFAULT_FILL_COLOR[0], DEFAULT_FILL_COLOR[1], DEFAULT_FILL_COLOR[2], DEFAULT_FILL_COLOR[3]),
        gradient_stops = None,
        gradient_angle = 0.0
    ))]
    fn new(
        uid: String,
        boundary: Vec<(EntityId, bool)>,
        style: PyFillStyle,
        color: Color4,
        gradient_stops: Option<Vec<(f64, Color4)>>,
        gradient_angle: f64,
    ) -> Self {
        PyFill {
            inner: CoreFill::new(
                uid,
                boundary.into_iter().map(|(id, f)| (id.0, f)).collect(),
                style.into(),
                color_to_array(color),
                gradient_stops
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(pos, color)| (pos, color_to_array(color)))
                    .collect(),
                gradient_angle,
            ),
        }
    }

    /// Unique fill id.
    #[getter]
    fn uid(&self) -> String {
        self.inner.uid.clone()
    }

    #[setter]
    fn set_uid(&mut self, value: String) {
        self.inner.uid = value;
    }

    /// Loop boundary as ``(entity_id, forward)`` pairs.
    #[getter]
    fn boundary(&self) -> Vec<(EntityId, bool)> {
        self.inner
            .boundary
            .iter()
            .map(|(id, forward)| (EntityId(*id), *forward))
            .collect()
    }

    #[setter]
    fn set_boundary(&mut self, value: Vec<(EntityId, bool)>) {
        self.inner.boundary =
            value.into_iter().map(|(id, f)| (id.0, f)).collect();
    }

    /// Fill rendering style.
    #[getter]
    fn style(&self) -> PyFillStyle {
        self.inner.style.into()
    }

    #[setter]
    fn set_style(&mut self, value: PyFillStyle) {
        self.inner.style = value.into();
    }

    /// Fill RGBA color.
    #[getter]
    fn color(&self) -> Color4 {
        color_from_array(self.inner.color)
    }

    #[setter]
    fn set_color(&mut self, value: Color4) {
        self.inner.color = color_to_array(value);
    }

    /// Gradient stops as ``(position, rgba)`` pairs.
    #[getter]
    fn gradient_stops(&self) -> Vec<(f64, Color4)> {
        self.inner
            .gradient_stops
            .iter()
            .map(|(pos, color)| (*pos, color_from_array(*color)))
            .collect()
    }

    #[setter]
    fn set_gradient_stops(&mut self, value: Vec<(f64, Color4)>) {
        self.inner.gradient_stops = value
            .into_iter()
            .map(|(pos, color)| (pos, color_to_array(color)))
            .collect();
    }

    /// Gradient angle in radians.
    #[getter]
    fn gradient_angle(&self) -> f64 {
        self.inner.gradient_angle
    }

    #[setter]
    fn set_gradient_angle(&mut self, value: f64) {
        self.inner.gradient_angle = value;
    }

    /// Serializes the fill to the reference dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        let dict = PyDict::new(py);
        if let serde_json::Value::Object(map) = value {
            for (key, item) in map {
                dict.set_item(key, value_to_py(py, &item)?)?;
            }
        }
        Ok(dict)
    }

    /// Deserializes a fill from the reference dictionary shape.
    #[staticmethod]
    fn from_dict(py: Python<'_>, dict: &Bound<'_, PyDict>) -> PyResult<Self> {
        let value = value_from_dict(dict)?;
        let mut inner = CoreFill::from_json(&value).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("invalid fill dict")
        })?;
        if inner.uid.is_empty() {
            let uuid = py.import("uuid")?;
            inner.uid = uuid.call_method0("uuid4")?.str()?.to_string();
        }
        Ok(PyFill { inner })
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let document_mod = PyModule::new(py, "document")?;

    document_mod
        .setattr("__doc__", "Sketch document data shared with the host.")?;
    document_mod.add_class::<PyFillStyle>()?;
    document_mod.add_class::<PyFill>()?;
    document_mod.setattr("FORMAT_VERSION", FORMAT_VERSION)?;
    document_mod.setattr(
        "DEFAULT_FILL_COLOR",
        (
            DEFAULT_FILL_COLOR[0],
            DEFAULT_FILL_COLOR[1],
            DEFAULT_FILL_COLOR[2],
            DEFAULT_FILL_COLOR[3],
        ),
    )?;

    m.add_submodule(&document_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.document", &document_mod)?;

    Ok(())
}

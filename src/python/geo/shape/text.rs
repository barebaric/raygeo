use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::geo::shape::text::{
    get_font_metrics as core_get_font_metrics,
    get_text_position as core_get_text_position,
    get_text_width as core_get_text_width, text_to_geometry,
    FontConfig as CoreFontConfig,
};

pub(crate) fn register(geo_shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = geo_shape_mod.py();
    let m = PyModule::new(py, "text")?;
    m.add_class::<PyFontConfig>()?;
    m.add_function(wrap_pyfunction!(text_to_geometry_py, &m)?)?;
    geo_shape_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.text", &m)?;

    Ok(())
}

fn known_keys() -> &'static [&'static str] {
    &["font_family", "font_size", "bold", "italic"]
}

#[gen_stub_pyclass(module = "raygeo.geo.shape.text")]
#[pyclass(
    skip_from_py_object,
    module = "raygeo.geo.shape.text",
    name = "FontConfig"
)]
#[derive(Debug)]
pub struct PyFontConfig {
    pub core: CoreFontConfig,
    pub extra: Py<PyDict>,
}

// Py<T> Clone increments the Python ref count — safe even though
// PyDict is !Clone in Rust.
impl Clone for PyFontConfig {
    fn clone(&self) -> Self {
        PyFontConfig {
            core: self.core.clone(),
            extra: self.extra.clone(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyFontConfig {
    #[new]
    #[pyo3(signature = (family = "sans-serif".to_string(), size = 10.0, bold = false, italic = false))]
    fn new(
        py: Python<'_>,
        family: String,
        size: f64,
        bold: bool,
        italic: bool,
    ) -> Self {
        PyFontConfig {
            core: CoreFontConfig {
                family,
                size,
                bold,
                italic,
            },
            extra: PyDict::new(py).into(),
        }
    }

    #[getter]
    fn family(&self) -> String {
        self.core.family.clone()
    }

    #[getter]
    fn size(&self) -> f64 {
        self.core.size
    }

    #[getter]
    fn bold(&self) -> bool {
        self.core.bold
    }

    #[getter]
    fn italic(&self) -> bool {
        self.core.italic
    }

    #[getter]
    fn font_family(&self) -> String {
        self.core.family.clone()
    }

    #[getter]
    fn font_size(&self) -> f64 {
        self.core.size
    }

    #[getter]
    fn extra(&self) -> Py<PyDict> {
        self.extra.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "FontConfig(family='{}', size={}, bold={}, italic={})",
            self.core.family, self.core.size, self.core.bold, self.core.italic,
        )
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        d.set_item("font_family", self.core.family.as_str())?;
        d.set_item("font_size", self.core.size)?;
        d.set_item("bold", self.core.bold)?;
        d.set_item("italic", self.core.italic)?;
        let extra = self.extra.bind(py);
        for (k, v) in extra.iter() {
            let key: String = k.extract()?;
            if !known_keys().contains(&key.as_str()) {
                d.set_item(k, v)?;
            }
        }
        Ok(d)
    }

    #[staticmethod]
    fn from_dict(
        py: Python<'_>,
        data: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let d = match data {
            Some(d) => d,
            None => {
                return Ok(PyFontConfig {
                    core: CoreFontConfig::new("sans-serif", 10.0),
                    extra: PyDict::new(py).into(),
                })
            }
        };
        let family = d
            .get_item("font_family")
            .ok()
            .flatten()
            .and_then(|v| v.extract::<String>().ok())
            .unwrap_or_else(|| "sans-serif".to_string());
        let size = d
            .get_item("font_size")
            .ok()
            .flatten()
            .and_then(|v| v.extract::<f64>().ok())
            .unwrap_or(10.0);
        let bold = d
            .get_item("bold")
            .ok()
            .flatten()
            .and_then(|v| v.extract::<bool>().ok())
            .unwrap_or(false);
        let italic = d
            .get_item("italic")
            .ok()
            .flatten()
            .and_then(|v| v.extract::<bool>().ok())
            .unwrap_or(false);
        let extra = PyDict::new(py);
        for (k, v) in d.iter() {
            let key: String = k.extract()?;
            if !known_keys().contains(&key.as_str()) {
                extra.set_item(k, v)?;
            }
        }
        Ok(PyFontConfig {
            core: CoreFontConfig {
                family,
                size,
                bold,
                italic,
            },
            extra: extra.into(),
        })
    }

    fn copy(&self, py: Python<'_>) -> Self {
        let new_extra = PyDict::new(py);
        for (k, v) in self.extra.bind(py).iter() {
            if let (Ok(key), Ok(val)) =
                (k.extract::<String>(), v.extract::<String>())
            {
                let _ = new_extra.set_item(&key, &val);
            }
        }
        PyFontConfig {
            core: self.core.clone(),
            extra: new_extra.into(),
        }
    }

    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other_fc) = other.clone().cast::<PyFontConfig>() {
            let other = other_fc.borrow();
            if self.core.family != other.core.family
                || self.core.size != other.core.size
                || self.core.bold != other.core.bold
                || self.core.italic != other.core.italic
            {
                return false;
            }
            let a_items: std::collections::BTreeMap<String, String> = self
                .extra
                .bind(py)
                .iter()
                .filter_map(|(k, v)| {
                    let k = k.extract::<String>().ok()?;
                    let v = v.extract::<String>().ok()?;
                    Some((k, v))
                })
                .collect();
            let b_items: std::collections::BTreeMap<String, String> = other
                .extra
                .bind(py)
                .iter()
                .filter_map(|(k, v)| {
                    let k = k.extract::<String>().ok()?;
                    let v = v.extract::<String>().ok()?;
                    Some((k, v))
                })
                .collect();
            a_items == b_items
        } else {
            false
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.core.family.hash(&mut h);
        (self.core.size as u64).hash(&mut h);
        self.core.bold.hash(&mut h);
        self.core.italic.hash(&mut h);
        h.finish()
    }

    fn get_text_width(&self, text: &str) -> f64 {
        core_get_text_width(text, &self.core).unwrap_or(0.0)
    }

    fn get_text_position(&self, text: &str, index: usize) -> f64 {
        core_get_text_position(text, index, &self.core).unwrap_or(0.0)
    }

    fn get_font_metrics(&self) -> (f64, f64, f64) {
        core_get_font_metrics(&self.core).unwrap_or((0.0, 0.0, 0.0))
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def text_to_geometry(
        text: str,
        font_config: raygeo.geo.shape.text.FontConfig = FontConfig(),
    ) -> raygeo.geo.Geometry:
        """Convert a text string to a Geometry containing the glyph outlines."""
    "#,
    module = "raygeo.geo.shape.text"
)]
#[pyfunction(name = "text_to_geometry")]
#[pyo3(signature = (text, font_config = None))]
fn text_to_geometry_py(
    text: &str,
    font_config: Option<&PyFontConfig>,
) -> PyResult<crate::python::geo::geometry::Geometry> {
    let fc = font_config
        .map(|f| f.core.clone())
        .unwrap_or_else(|| CoreFontConfig::new("sans-serif", 10.0));
    match text_to_geometry(text, &fc) {
        Some(geo) => Ok(crate::python::geo::geometry::Geometry { inner: geo }),
        None => Ok(crate::python::geo::geometry::Geometry {
            inner: crate::geo::Geometry::new(),
        }),
    }
}

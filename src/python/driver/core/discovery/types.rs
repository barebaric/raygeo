//! Python-facing discovery result types.
//!
//! Thin wrappers over the protocol-neutral core types so embedding
//! applications consume scan results directly. ``probe_profile`` is
//! an opaque handle attached by the embedding application's UI; the
//! core never inspects it.

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFrozenSet};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::discovery::types::{
    DeviceIdentity as CoreIdentity, DiscoveredDevice as CoreDevice, ParamValue,
};

pub(crate) fn params_from_py(
    dict: &Bound<'_, PyDict>,
) -> PyResult<HashMap<String, ParamValue>> {
    let mut params = HashMap::new();
    for (key, value) in dict.iter() {
        let key: String = key.extract()?;
        if let Ok(text) = value.extract::<String>() {
            params.insert(key, ParamValue::Str(text));
        } else {
            params.insert(key, ParamValue::Int(value.extract()?));
        }
    }
    Ok(params)
}

fn params_to_py<'py>(
    py: Python<'py>,
    params: &HashMap<String, ParamValue>,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    for (key, value) in params {
        match value {
            ParamValue::Str(text) => dict.set_item(key, text)?,
            ParamValue::Int(number) => dict.set_item(key, number)?,
        }
    }
    Ok(dict)
}

/// Everything known about a discovered device that can be used to
/// match it against a known device profile.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery")]
#[pyclass(
    module = "raygeo.driver.core.discovery",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DeviceIdentity {
    pub(crate) inner: CoreIdentity,
}

#[gen_stub_pymethods]
#[pymethods]
impl DeviceIdentity {
    #[new]
    #[pyo3(signature = (firmware=None, banner=None, usb_vid=None, usb_pid=None, tokens=None))]
    fn new(
        firmware: Option<String>,
        banner: Option<String>,
        usb_vid: Option<u16>,
        usb_pid: Option<u16>,
        #[gen_stub(override_type(type_repr = "typing.Optional[typing.AbstractSet[builtins.str]]", imports = ("typing", "builtins")))]
        tokens: Option<HashSet<String>>,
    ) -> Self {
        Self {
            inner: CoreIdentity {
                firmware,
                banner,
                usb_vid,
                usb_pid,
                tokens: tokens.unwrap_or_default().into_iter().collect(),
            },
        }
    }

    #[getter]
    fn firmware(&self) -> Option<String> {
        self.inner.firmware.clone()
    }

    #[getter]
    fn banner(&self) -> Option<String> {
        self.inner.banner.clone()
    }

    #[getter]
    fn usb_vid(&self) -> Option<u16> {
        self.inner.usb_vid
    }

    #[getter]
    fn usb_pid(&self) -> Option<u16> {
        self.inner.usb_pid
    }

    /// The identity tokens as a frozenset of strings.
    #[getter]
    #[gen_stub(override_return_type(type_repr = "builtins.frozenset[builtins.str]", imports = ("builtins",)))]
    fn tokens(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(PyFrozenSet::new(py, self.inner.tokens.iter())?
            .unbind()
            .into_any())
    }
}

/// A device found by evaluating scan output against a driver's
/// discovery declaration.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery")]
#[pyclass(module = "raygeo.driver.core.discovery", skip_from_py_object)]
#[derive(Clone)]
pub struct DiscoveredDevice {
    pub(crate) inner: CoreDevice,
    /// Profile data collected by probing the device after discovery;
    /// attached by the embedding application's UI. Opaque here.
    pub(crate) probe_profile: Option<Py<PyAny>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl DiscoveredDevice {
    #[new]
    #[pyo3(signature = (driver_name, params, label, detail, identity=None, probe_profile=None))]
    fn new(
        driver_name: String,
        #[gen_stub(override_type(type_repr = "dict[builtins.str, builtins.object]", imports = ("builtins",)))]
        params: &Bound<'_, PyDict>,
        label: String,
        detail: String,
        identity: Option<PyRef<'_, DeviceIdentity>>,
        #[gen_stub(override_type(type_repr = "typing.Optional[builtins.object]", imports = ("typing", "builtins")))]
        probe_profile: Option<Py<PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: CoreDevice {
                driver_name,
                params: params_from_py(params)?,
                label,
                detail,
                identity: identity.map(|i| i.inner.clone()).unwrap_or_default(),
            },
            probe_profile,
        })
    }

    #[getter]
    fn driver_name(&self) -> String {
        self.inner.driver_name.clone()
    }

    #[getter]
    #[gen_stub(override_return_type(type_repr = "dict[builtins.str, builtins.object]", imports = ("builtins",)))]
    fn params<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        params_to_py(py, &self.inner.params)
    }

    #[getter]
    fn label(&self) -> String {
        self.inner.label.clone()
    }

    #[getter]
    fn detail(&self) -> String {
        self.inner.detail.clone()
    }

    #[getter]
    fn identity(&self) -> DeviceIdentity {
        DeviceIdentity {
            inner: self.inner.identity.clone(),
        }
    }

    /// Stable identity for deduplication in the UI.
    #[getter]
    fn key(&self) -> String {
        self.inner.key()
    }

    /// The machine name the device itself reported, if any.
    #[getter]
    fn probe_name(&self, py: Python<'_>) -> PyResult<Option<String>> {
        let Some(profile) = &self.probe_profile else {
            return Ok(None);
        };
        let meta = profile.getattr(py, "meta")?;
        let name: Option<String> = meta.getattr(py, "name")?.extract(py)?;
        let name = name.unwrap_or_default().trim().to_string();
        if name.is_empty() || name.starts_with("Unknown") {
            return Ok(None);
        }
        Ok(Some(name))
    }

    /// Profile data collected by probing the device after discovery;
    /// attached by the embedding application's UI. Opaque here: the
    /// embedding application's profile object, not a core type.
    #[getter]
    #[gen_stub(override_return_type(type_repr = "typing.Optional[builtins.object]", imports = ("typing", "builtins")))]
    fn probe_profile(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.probe_profile.as_ref().map(|p| p.clone_ref(py))
    }

    /// Profile data collected by probing the device after discovery;
    /// attached by the embedding application's UI. Opaque here: the
    /// embedding application's profile object, not a core type.
    #[setter]
    fn set_probe_profile(
        &mut self,
        #[gen_stub(override_type(type_repr = "typing.Optional[builtins.object]", imports = ("typing", "builtins")))]
        value: Option<Py<PyAny>>,
    ) {
        self.probe_profile = value;
    }

    #[gen_stub(override_return_type(type_repr = "builtins.bool", imports = ("builtins",)))]
    fn __eq__(
        &self,
        #[gen_stub(override_type(type_repr = "builtins.object", imports = ("builtins",)))]
        other: &Bound<'_, PyAny>,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let Ok(other) = other.extract::<PyRef<'_, DiscoveredDevice>>() else {
            return Ok(py.NotImplemented());
        };
        if self.inner != other.inner {
            return Ok(false
                .into_pyobject(py)?
                .as_unbound()
                .clone_ref(py)
                .into_any());
        }
        let equal = match (&self.probe_profile, &other.probe_profile) {
            (None, None) => true,
            (Some(a), Some(b)) => a.bind(py).eq(b.bind(py))?,
            _ => false,
        };
        Ok(equal
            .into_pyobject(py)?
            .as_unbound()
            .clone_ref(py)
            .into_any())
    }

    fn __repr__(&self) -> String {
        format!(
            "DiscoveredDevice(driver_name={:?}, params={:?}, label={:?})",
            self.inner.driver_name, self.inner.params, self.inner.label
        )
    }
}

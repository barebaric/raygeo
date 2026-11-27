//! The `raydriver.core.discovery.spec` submodule: driver-facing
//! discovery declarations.
//!
//! A driver participates in automatic device discovery by declaring
//! a single `DiscoverySpec` as its `DISCOVERY` class attribute. The
//! spec is composed of per-channel recognizers; the recognition
//! hooks (label, matches, name, fingerprint) are driver-supplied
//! callables, evaluated by the Rust channels.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::helpers;

/// Declarative description of what a driver's device looks like on
/// a serial wire. Evaluated against captured port output by the
/// serial channel.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.spec")]
#[pyclass(
    module = "raygeo.driver.core.discovery.spec",
    frozen,
    skip_from_py_object
)]
pub struct SerialRecognizer {
    /// Display label, called at discovery time so translations are
    /// looked up at runtime.
    label: Py<PyAny>,
    /// Returns True if the bytes captured from a port were produced
    /// by this driver's firmware.
    matches: Py<PyAny>,
    /// Optional: extracts a human-readable identity hint from the
    /// captured bytes — e.g. a firmware version banner. The result
    /// becomes the identity banner and feeds profile matching.
    name: Option<Py<PyAny>>,
    /// Firmware identifier used for identity/profile matching.
    firmware: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl SerialRecognizer {
    #[new]
    #[pyo3(signature = (label, matches, name=None, firmware=None))]
    fn new(
        #[gen_stub(override_type(type_repr = "typing.Callable[[], builtins.str]", imports = ("typing", "builtins")))]
        label: Py<PyAny>,
        #[gen_stub(override_type(type_repr = "typing.Callable[[builtins.bytes], builtins.bool]", imports = ("typing", "builtins")))]
        matches: Py<PyAny>,
        #[gen_stub(override_type(type_repr = "typing.Optional[typing.Callable[[builtins.bytes], typing.Optional[builtins.str]]]", imports = ("typing", "builtins")))]
        name: Option<Py<PyAny>>,
        firmware: Option<String>,
    ) -> Self {
        Self {
            label,
            matches,
            name,
            firmware,
        }
    }

    #[getter]
    #[gen_stub(override_return_type(type_repr = "typing.Callable[[], builtins.str]", imports = ("typing", "builtins")))]
    pub(crate) fn label(&self, py: Python<'_>) -> Py<PyAny> {
        self.label.clone_ref(py)
    }

    #[getter]
    #[gen_stub(override_return_type(type_repr = "typing.Callable[[builtins.bytes], builtins.bool]", imports = ("typing", "builtins")))]
    pub(crate) fn matches(&self, py: Python<'_>) -> Py<PyAny> {
        self.matches.clone_ref(py)
    }

    #[getter]
    #[gen_stub(override_return_type(type_repr = "typing.Optional[typing.Callable[[builtins.bytes], typing.Optional[builtins.str]]]", imports = ("typing", "builtins")))]
    pub(crate) fn name(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.name.as_ref().map(|n| n.clone_ref(py))
    }

    #[getter]
    pub(crate) fn firmware(&self) -> Option<String> {
        self.firmware.clone()
    }
}

/// Declarative description of how a driver's devices appear on the
/// network. Evaluated against resolved mDNS services by the network
/// channel.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.spec")]
#[pyclass(
    module = "raygeo.driver.core.discovery.spec",
    frozen,
    skip_from_py_object
)]
pub struct MdnsRecognizer {
    /// mDNS service types announcing devices this driver can talk
    /// to, e.g. ``("_esp3d._tcp.local.",)``.
    services: Vec<String>,
    /// Maps mDNS TXT record keys to the driver-arg keys they are
    /// forwarded as into the discovered device's params.
    txt_map: HashMap<String, String>,
    /// Service types whose unclaimed announcements are offered to
    /// this driver's ``fingerprint`` probe. Defaults to
    /// ``("_http._tcp",)`` so existing declarations keep their
    /// behavior.
    fingerprint_services: Vec<String>,
    /// Optional async probe for generic service types (e.g.
    /// ``_http._tcp``): candidates are fingerprinted to confirm
    /// their firmware before they are claimed. Must be bounded and
    /// never raise.
    fingerprint: Option<Py<PyAny>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MdnsRecognizer {
    #[new]
    #[pyo3(signature = (
        services=None,
        txt_map=None,
        fingerprint=None,
        fingerprint_services=None
    ))]
    fn new(
        #[gen_stub(override_type(type_repr = "typing.Optional[typing.Tuple[builtins.str, ...]]", imports = ("typing", "builtins")))]
        services: Option<Vec<String>>,
        txt_map: Option<HashMap<String, String>>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[collections.abc.Callable[..., \
                         collections.abc.Awaitable[builtins.bool]]]",
            imports = ("typing", "collections.abc", "builtins"),
        ))]
        fingerprint: Option<Py<PyAny>>,
        #[gen_stub(override_type(type_repr = "typing.Optional[typing.Tuple[builtins.str, ...]]", imports = ("typing", "builtins")))]
        fingerprint_services: Option<Vec<String>>,
    ) -> Self {
        Self {
            services: services.unwrap_or_default(),
            txt_map: txt_map.unwrap_or_default(),
            fingerprint,
            fingerprint_services: fingerprint_services
                .unwrap_or_else(|| vec!["_http._tcp".to_string()]),
        }
    }

    #[getter]
    fn services<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, &self.services)
    }

    #[getter]
    fn txt_map<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.txt_map {
            dict.set_item(key, value)?;
        }
        Ok(dict)
    }

    #[getter]
    #[gen_stub(override_return_type(
        type_repr = "typing.Optional[collections.abc.Callable[..., \
                     collections.abc.Awaitable[builtins.bool]]]",
        imports = ("typing", "collections.abc", "builtins"),
    ))]
    fn fingerprint(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.fingerprint.as_ref().map(|f| f.clone_ref(py))
    }

    #[getter]
    fn fingerprint_services<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, &self.fingerprint_services)
    }
}

/// Declarative description of how a driver's devices appear as plain
/// TCP listeners on the local network — e.g. grblHAL's raw telnet
/// service on boards whose firmware does not announce it via mDNS.
/// The subnet scan probes reachable hosts on the declared port with
/// this data.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.spec")]
#[pyclass(
    module = "raygeo.driver.core.discovery.spec",
    frozen,
    skip_from_py_object
)]
pub struct TcpRecognizer {
    /// TCP port the device listens on.
    port: u16,
    /// Command sent to confirm the firmware on a reachable host.
    /// Without it, any listener on the port is claimed.
    probe_command: Option<String>,
    /// Response-line prefix identifying the firmware; the matched
    /// line becomes the device's identity banner.
    match_prefix: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl TcpRecognizer {
    #[new]
    #[pyo3(signature = (port=23, probe_command=None, match_prefix=None))]
    fn new(
        port: u16,
        probe_command: Option<String>,
        match_prefix: Option<String>,
    ) -> Self {
        Self {
            port,
            probe_command,
            match_prefix,
        }
    }

    #[getter]
    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    #[getter]
    pub(crate) fn probe_command(&self) -> Option<String> {
        self.probe_command.clone()
    }

    #[getter]
    pub(crate) fn match_prefix(&self) -> Option<String> {
        self.match_prefix.clone()
    }
}

/// Everything a driver declares about discoverability. Declared as
/// the ``DISCOVERY`` class attribute; evaluated by the discovery
/// channels.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.spec")]
#[pyclass(
    module = "raygeo.driver.core.discovery.spec",
    frozen,
    skip_from_py_object
)]
pub struct DiscoverySpec {
    serial: Option<Py<SerialRecognizer>>,
    mdns: Option<Py<MdnsRecognizer>>,
    tcp: Option<Py<TcpRecognizer>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl DiscoverySpec {
    #[new]
    #[pyo3(signature = (serial=None, mdns=None, tcp=None))]
    fn new(
        serial: Option<Py<SerialRecognizer>>,
        mdns: Option<Py<MdnsRecognizer>>,
        tcp: Option<Py<TcpRecognizer>>,
    ) -> Self {
        Self { serial, mdns, tcp }
    }

    #[getter]
    fn serial(&self, py: Python<'_>) -> Option<Py<SerialRecognizer>> {
        self.serial.as_ref().map(|s| s.clone_ref(py))
    }

    #[getter]
    pub(crate) fn mdns(&self, py: Python<'_>) -> Option<Py<MdnsRecognizer>> {
        self.mdns.as_ref().map(|m| m.clone_ref(py))
    }

    #[getter]
    pub(crate) fn tcp(&self, py: Python<'_>) -> Option<Py<TcpRecognizer>> {
        self.tcp.as_ref().map(|t| t.clone_ref(py))
    }
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "spec")?;
    module.setattr(
        "__doc__",
        "Driver-facing discovery declarations (DiscoverySpec and \
         per-channel recognizers).",
    )?;
    module.add_class::<SerialRecognizer>()?;
    module.add_class::<MdnsRecognizer>()?;
    module.add_class::<TcpRecognizer>()?;
    module.add_class::<DiscoverySpec>()?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.spec",
    )?;
    Ok(())
}

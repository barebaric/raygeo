//! The `raygeo.driver.core.discovery.engine` submodule: discovery engine.
//!
//! Ties the channels together: `find_all_devices` runs every channel's
//! scan concurrently and merges the results. Each channel owns its
//! scanner, its recognizer type (declared in a driver's
//! `DiscoverySpec`) and its timeout/error handling, so adding a
//! channel never changes the engine or the driver base class.

use std::collections::HashSet;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use pyo3_stub_gen::{PyStubType, TypeInfo};

use crate::python::driver::pyfuture::{await_py, spawn_future, PyAwait};

use super::helpers;
use super::spec::{DiscoverySpec, TcpRecognizer};
use super::types::DiscoveredDevice;

/// A replaceable device-scan callable: the channels resolve their
/// collaborators from their module namespace at call time (see
/// helpers), so the engine's scanner attributes are typed as async
/// callables resolving to the discovered devices.
pub(crate) struct ScanDevicesFn;

impl PyStubType for ScanDevicesFn {
    fn type_output() -> TypeInfo {
        let device = DiscoveredDevice::type_output();
        let mut import = device.import.clone();
        import.insert("collections.abc".into());
        import.insert("builtins".into());
        TypeInfo {
            name: format!(
                "collections.abc.Callable[..., \
                 collections.abc.Awaitable[builtins.list[{}]]]",
                device.name
            ),
            source_module: None,
            import,
            type_refs: device.type_refs,
        }
    }
}

const LOGGER: &str = "raygeo.driver.core.discovery.engine";

/// Upper bound for one subnet scan plus its confirmation pass.
const SUBNET_SCAN_TIMEOUT: f64 = 30.0;

/// Runs every discovery channel concurrently and returns the merged,
/// deduplicated results.
///
/// *exclude_ports* keeps the serial scan away from ports whose
/// devices are already held (e.g. being probed by the caller).
/// Channels never raise: a failing or slow scan yields no devices
/// from that channel only.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.engine")]
#[pyfunction]
#[pyo3(signature = (driver_classes, ports=None, baud_rates=None, exclude_ports=None))]
fn find_all_devices<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(type_repr = "typing.Sequence[builtins.type]", imports = ("typing", "builtins")))]
    driver_classes: Vec<Bound<'_, PyAny>>,
    #[gen_stub(override_type(type_repr = "typing.Optional[typing.Iterable[builtins.str]]", imports = ("typing", "builtins")))]
    ports: Option<Bound<'_, PyAny>>,
    #[gen_stub(override_type(type_repr = "typing.Optional[typing.Iterable[builtins.int]]", imports = ("typing", "builtins")))]
    baud_rates: Option<Bound<'_, PyAny>>,
    #[gen_stub(override_type(type_repr = "typing.Optional[typing.Iterable[builtins.str]]", imports = ("typing", "builtins")))]
    exclude_ports: Option<Bound<'_, PyAny>>,
) -> PyResult<PyAwait<'py, Vec<Py<DiscoveredDevice>>>> {
    let serial = helpers::attr(py, "engine", "find_serial_devices")?;
    let network = helpers::attr(py, "engine", "find_network_devices")?;
    let raydriver = helpers::attr(py, "engine", "find_raygeo_devices")?;
    let subnet = helpers::attr(py, "engine", "find_subnet_devices")?;

    let serial_fut = await_py(
        py,
        serial.call1((
            driver_classes.clone(),
            ports,
            baud_rates,
            exclude_ports,
        ))?,
    )?;
    let network_fut = await_py(py, network.call1((driver_classes.clone(),))?)?;
    let raydriver_fut = await_py(py, raydriver.call0()?)?;
    let subnet_fut = await_py(py, subnet.call1((driver_classes,))?)?;

    spawn_future(py, async move {
        let (serial, network, raydriver, subnet) =
            tokio::join!(serial_fut, network_fut, raydriver_fut, subnet_fut);
        Python::attach(|py| {
            let mut devices: Vec<Py<DiscoveredDevice>> = Vec::new();
            for result in [serial, network, raydriver, subnet] {
                let found: Vec<Py<DiscoveredDevice>> =
                    result?.bind(py).extract()?;
                devices.extend(found);
            }
            dedupe(py, devices)
        })
    })
}

/// Scans the local subnets for the TCP ports declared by drivers'
/// `TcpRecognizer` and returns the devices the Rust core confirmed
/// with each driver's probe. Finds controllers that announce nothing
/// via mDNS, such as grblHAL boards with the raw telnet service.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.engine")]
#[pyfunction]
fn find_subnet_devices<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(type_repr = "typing.Sequence[builtins.type]", imports = ("typing", "builtins")))]
    driver_classes: Vec<Bound<'_, PyAny>>,
) -> PyResult<PyAwait<'py, Vec<Py<DiscoveredDevice>>>> {
    let mut specs = Vec::new();
    for cls in &driver_classes {
        let Ok(spec) = cls.getattr("DISCOVERY") else {
            continue;
        };
        if !spec.is_instance_of::<DiscoverySpec>() {
            continue;
        }
        let spec: PyRef<'_, DiscoverySpec> = spec.extract()?;
        let Some(tcp) = spec.tcp(py) else {
            continue;
        };
        let tcp: PyRef<'_, TcpRecognizer> = tcp.bind(py).extract()?;
        let name: String = cls.getattr("__name__")?.extract()?;
        let dict = PyDict::new(py);
        dict.set_item("id", &name)?;
        dict.set_item("port", tcp.port())?;
        dict.set_item("probe_command", tcp.probe_command())?;
        dict.set_item("match_prefix", tcp.match_prefix())?;
        dict.set_item("driver_name", &name)?;
        dict.set_item("label", cls.getattr("label")?)?;
        dict.set_item(
            "firmware",
            name.strip_suffix("Driver").unwrap_or(&name).to_lowercase(),
        )?;
        specs.push(dict);
    }
    if specs.is_empty() {
        return spawn_future(py, async move {
            let empty: Vec<Py<DiscoveredDevice>> = Vec::new();
            Ok(empty)
        });
    }

    let timeout = helpers::timeout_attr(py, "engine", "SUBNET_SCAN_TIMEOUT")?;
    let scan = helpers::attr(py, "engine", "_rust_find_subnet_devices")?;
    let scan_fut = helpers::wait_for(py, scan.call1((specs,))?, timeout)?;
    spawn_future(py, async move {
        let outcome = scan_fut.await;
        Python::attach(|py| match outcome {
            Ok(value) => value.bind(py).extract::<Vec<Py<DiscoveredDevice>>>(),
            Err(e) => {
                if helpers::is_timeout(py, &e) {
                    helpers::log_warning(
                        py,
                        LOGGER,
                        "Subnet device discovery timed out",
                    );
                } else {
                    helpers::log_exception(
                        py,
                        LOGGER,
                        "Subnet device discovery failed",
                        &e,
                    );
                }
                let empty: Vec<Py<DiscoveredDevice>> = Vec::new();
                Ok(empty)
            }
        })
    })
}

/// Drops duplicate device keys, keeping the first occurrence.
fn dedupe(
    py: Python<'_>,
    devices: Vec<Py<DiscoveredDevice>>,
) -> PyResult<Vec<Py<DiscoveredDevice>>> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for device in devices {
        let key: String = device.getattr(py, "key")?.extract(py)?;
        if seen.insert(key) {
            unique.push(device);
        }
    }
    Ok(unique)
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "engine")?;
    module.setattr(
        "__doc__",
        "Discovery engine: runs every channel concurrently and \
         merges the results.",
    )?;
    module.setattr("SUBNET_SCAN_TIMEOUT", SUBNET_SCAN_TIMEOUT)?;
    // Channel collaborators, replaceable as module attributes.
    let serial = helpers::submodule(discovery_mod, "serial_channel")?;
    module.setattr(
        "find_serial_devices",
        serial.getattr("find_serial_devices")?,
    )?;
    let network = helpers::submodule(discovery_mod, "mdns_channel")?;
    module.setattr(
        "find_network_devices",
        network.getattr("find_network_devices")?,
    )?;
    let raydriver = helpers::submodule(discovery_mod, "raygeo_channel")?;
    module.setattr(
        "find_raygeo_devices",
        raydriver.getattr("find_raygeo_devices")?,
    )?;
    module.setattr(
        "_rust_find_subnet_devices",
        discovery_mod.getattr("find_subnet_devices")?,
    )?;
    module.add_function(wrap_pyfunction!(find_all_devices, &module)?)?;
    module.add_function(wrap_pyfunction!(find_subnet_devices, &module)?)?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.engine",
    )?;
    Ok(())
}
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.engine",
    "SUBNET_SCAN_TIMEOUT",
    f64
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.engine",
    "find_serial_devices",
    ScanDevicesFn
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.engine",
    "find_network_devices",
    ScanDevicesFn
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.engine",
    "find_raygeo_devices",
    ScanDevicesFn
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.engine",
    "_rust_find_subnet_devices",
    ScanDevicesFn
);

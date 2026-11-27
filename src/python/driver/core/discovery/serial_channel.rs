//! The `raygeo.driver.core.discovery.serial_channel` submodule: serial
//! discovery channel.
//!
//! Runs one serial scan per discovery run and evaluates the captured
//! port observations against every driver's `SerialRecognizer`.
//! Recognition is a pure function over the captured bytes; no port is
//! ever opened on behalf of a single driver, so adding drivers never
//! multiplies scan cost.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFrozenSet, PyList, PyTuple};
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::driver::core::discovery::tokens;
use crate::driver::core::discovery::types::{
    DiscoveredDevice as CoreDevice, ParamValue,
};
use crate::python::driver::pyfuture::{await_py, spawn_future, PyAwait};

use super::helpers;
use super::ports::SerialPortInfo;
use super::serial_scan::{PortObservation, ScanSerialFn};
use super::spec::{DiscoverySpec, SerialRecognizer};
use super::types::{DeviceIdentity, DiscoveredDevice};

const LOGGER: &str = "raygeo.driver.core.discovery.serial_channel";

/// Time budget for the serial part of a discovery scan. Ports that
/// have answered before it is hit are still reported; only probes
/// still in flight at that moment are dropped.
const SERIAL_SCAN_TIMEOUT: f64 = 20.0;

/// Splits arbitrary identification strings (descriptions, by-id link
/// names, manufacturer strings) into lowercase word tokens suitable
/// for containment checks.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.serial_channel")]
#[pyfunction]
#[pyo3(signature = (*texts))]
#[gen_stub(override_return_type(type_repr = "builtins.frozenset[builtins.str]", imports = ("builtins",)))]
fn normalize_tokens(
    py: Python<'_>,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[builtins.str]",
        imports = ("typing", "builtins")
    ))]
    texts: &Bound<'_, PyTuple>,
) -> PyResult<Py<PyAny>> {
    let texts: Vec<Option<String>> = texts.extract()?;
    let refs: Vec<Option<&str>> = texts.iter().map(|t| t.as_deref()).collect();
    let words = tokens::normalize_tokens(&refs);
    Ok(PyFrozenSet::new(py, words.iter())?.unbind().into_any())
}

/// Assembles a `DeviceIdentity` from serial scan artifacts. A
/// *device_name* extracted from the device's own output (when the
/// recognizer provides one) is the identity banner and contributes
/// matching tokens.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.serial_channel")]
#[pyfunction]
#[pyo3(signature = (firmware, port_info, device_name=None))]
fn build_identity(
    firmware: Option<String>,
    port_info: Option<PyRef<'_, SerialPortInfo>>,
    device_name: Option<String>,
) -> DeviceIdentity {
    let info = port_info.map(|i| i.inner.clone());
    DeviceIdentity {
        inner: tokens::build_identity(
            firmware.as_deref(),
            info.as_ref(),
            device_name.as_deref(),
        ),
    }
}

fn driver_name(cls: &Bound<'_, PyAny>) -> PyResult<String> {
    cls.getattr("__name__")?.extract()
}

fn discovery_spec<'py>(
    cls: &Bound<'py, PyAny>,
) -> PyResult<Option<Bound<'py, DiscoverySpec>>> {
    let Ok(spec) = cls.getattr("DISCOVERY") else {
        return Ok(None);
    };
    if spec.is_instance_of::<DiscoverySpec>() {
        Ok(Some(spec.extract()?))
    } else {
        Ok(None)
    }
}

/// Pairs every driver whose ``DISCOVERY`` spec declares a valid
/// serial recognizer with it.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.serial_channel")]
#[pyfunction]
#[gen_stub(override_return_type(
    type_repr = "builtins.list[tuple[builtins.type, \
                 spec.SerialRecognizer]]",
    imports = ("builtins", "raygeo.driver.core.discovery.spec"),
))]
fn collect_serial_recognizers<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(type_repr = "typing.Sequence[builtins.type]", imports = ("typing", "builtins")))]
    driver_classes: Vec<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty(py);
    for cls in driver_classes {
        let Some(spec) = discovery_spec(&cls)? else {
            continue;
        };
        let recognizer = spec.getattr("serial")?;
        if recognizer.is_none() {
            continue;
        }
        out.append(PyTuple::new(py, [cls, recognizer])?)?;
    }
    Ok(out)
}

/// Runs one serial scan and evaluates it against every declared
/// serial recognizer. Like every channel, this never raises: failures
/// are logged, and the scan is bounded by *SERIAL_SCAN_TIMEOUT*, so a
/// slow or silent adapter can at most cost its own probe time.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.serial_channel")]
#[pyfunction]
#[pyo3(signature = (driver_classes, ports=None, baud_rates=None, exclude_ports=None))]
fn find_serial_devices<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(type_repr = "typing.Sequence[builtins.type]", imports = ("typing", "builtins")))]
    driver_classes: Vec<Bound<'_, PyAny>>,
    #[gen_stub(override_type(type_repr = "typing.Optional[typing.Iterable[builtins.str]]", imports = ("typing", "builtins")))]
    ports: Option<Bound<'_, PyAny>>,
    #[gen_stub(override_type(type_repr = "typing.Optional[typing.Iterable[builtins.int]]", imports = ("typing", "builtins")))]
    baud_rates: Option<Bound<'_, PyAny>>,
    #[gen_stub(override_type(type_repr = "typing.Optional[typing.Iterable[builtins.str]]", imports = ("typing", "builtins")))]
    exclude_ports: Option<Bound<'_, PyAny>>,
) -> PyResult<PyAwait<'py, Vec<DiscoveredDevice>>> {
    let recognizers = collect_serial_recognizers(py, driver_classes)?;
    if recognizers.is_empty() {
        return spawn_future(py, async move {
            let empty: Vec<DiscoveredDevice> = Vec::new();
            Ok(empty)
        });
    }

    let timeout =
        helpers::timeout_attr(py, "serial_channel", "SERIAL_SCAN_TIMEOUT")?;
    let scan = helpers::attr(py, "serial_channel", "scan_serial_ports")?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("ports", ports)?;
    kwargs.set_item("baud_rates", baud_rates)?;
    kwargs.set_item("exclude_ports", exclude_ports)?;
    kwargs.set_item("timeout", timeout.as_secs_f64())?;
    let scan_fut = await_py(py, scan.call(PyTuple::empty(py), Some(&kwargs))?)?;

    // Owned recognizer data for the evaluation pass: the driver
    // callables plus the display fields read from them.
    let mut owned = Vec::new();
    for item in recognizers.iter() {
        let cls = item.get_item(0)?;
        let recognizer: PyRef<'_, SerialRecognizer> =
            item.get_item(1)?.extract()?;
        owned.push((
            driver_name(&cls)?,
            recognizer.label(py),
            recognizer.matches(py),
            recognizer.name(py),
            recognizer.firmware(),
        ));
    }

    spawn_future(py, async move {
        let outcome = scan_fut.await;
        Python::attach(|py| {
            let value = match outcome {
                Ok(value) => value,
                Err(e) => {
                    helpers::log_exception(
                        py,
                        LOGGER,
                        "Serial device discovery failed",
                        &e,
                    );
                    let empty: Vec<DiscoveredDevice> = Vec::new();
                    return Ok(empty);
                }
            };
            let observations: Vec<PyRef<'_, PortObservation>> =
                value.bind(py).extract()?;
            let mut devices = Vec::new();
            for observation in observations {
                devices.extend(evaluate(py, &observation, &owned)?);
            }
            Ok(devices)
        })
    })
}

type OwnedRecognizer = (
    String,
    Py<PyAny>,
    Py<PyAny>,
    Option<Py<PyAny>>,
    Option<String>,
);

/// Runs every recognizer against one observation.
fn evaluate(
    py: Python<'_>,
    observation: &PyRef<'_, PortObservation>,
    recognizers: &[OwnedRecognizer],
) -> PyResult<Vec<DiscoveredDevice>> {
    let mut devices = Vec::new();
    for (name, label, matches, extractor, firmware) in recognizers {
        let matched = matches.call1(py, (observation.data(py),));
        let matched = match matched {
            Ok(value) => value.is_truthy(py)?,
            Err(e) => {
                helpers::log_exception(
                    py,
                    LOGGER,
                    &format!("Discovery recognizer for {name} failed"),
                    &e,
                );
                continue;
            }
        };
        if !matched {
            continue;
        }
        devices.push(build_device(
            py,
            name,
            label,
            extractor.as_ref(),
            firmware.as_deref(),
            observation,
        )?);
    }
    Ok(devices)
}

fn build_device(
    py: Python<'_>,
    name: &str,
    label: &Py<PyAny>,
    extractor: Option<&Py<PyAny>>,
    firmware: Option<&str>,
    observation: &PyRef<'_, PortObservation>,
) -> PyResult<DiscoveredDevice> {
    let device_name: Option<String> = match extractor {
        Some(extractor) => {
            extractor.call1(py, (observation.data(py),))?.extract(py)?
        }
        None => None,
    };
    let label: String = label.call0(py)?.extract(py)?;
    let detail = helpers::tr(py, "{port} at {baud} baud")?
        .replace("{port}", &observation.port())
        .replace("{baud}", &observation.baud_rate().to_string());
    let mut params = HashMap::new();
    params.insert("port".to_string(), ParamValue::Str(observation.port()));
    params.insert(
        "baudrate".to_string(),
        ParamValue::Int(observation.baud_rate() as i64),
    );
    let info = observation.info();
    let identity = tokens::build_identity(
        firmware,
        info.as_ref().map(|i| &i.inner),
        device_name.as_deref(),
    );
    Ok(DiscoveredDevice {
        inner: CoreDevice {
            driver_name: name.to_string(),
            params,
            label,
            detail,
            identity,
        },
        probe_profile: None,
    })
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "serial_channel")?;
    module.setattr(
        "__doc__",
        "Serial discovery channel: one scan, evaluated against every \
         driver's serial recognizer.",
    )?;
    module.setattr("SERIAL_SCAN_TIMEOUT", SERIAL_SCAN_TIMEOUT)?;
    for (name, words) in [
        ("USB_CHIP_TOKENS", tokens::USB_CHIP_TOKENS),
        ("GENERIC_DEVICE_TOKENS", tokens::GENERIC_DEVICE_TOKENS),
        ("CORPORATE_TOKENS", tokens::CORPORATE_TOKENS),
    ] {
        module.setattr(name, PyFrozenSet::new(py, words.iter())?)?;
    }
    module.setattr(
        "GENERIC_TOKENS",
        PyFrozenSet::new(py, tokens::GENERIC_TOKENS.iter())?,
    )?;
    // The scanner backend, replaceable as a module attribute.
    let scan_mod = helpers::submodule(discovery_mod, "serial_scan")?;
    module
        .setattr("scan_serial_ports", scan_mod.getattr("scan_serial_ports")?)?;
    module.add_function(wrap_pyfunction!(normalize_tokens, &module)?)?;
    module.add_function(wrap_pyfunction!(build_identity, &module)?)?;
    module
        .add_function(wrap_pyfunction!(collect_serial_recognizers, &module)?)?;
    module.add_function(wrap_pyfunction!(find_serial_devices, &module)?)?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.serial_channel",
    )?;
    Ok(())
}
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.serial_channel",
    "SERIAL_SCAN_TIMEOUT",
    f64
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.serial_channel",
    "USB_CHIP_TOKENS",
    std::collections::HashSet<String>
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.serial_channel",
    "GENERIC_DEVICE_TOKENS",
    std::collections::HashSet<String>
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.serial_channel",
    "CORPORATE_TOKENS",
    std::collections::HashSet<String>
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.serial_channel",
    "GENERIC_TOKENS",
    std::collections::HashSet<String>
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.serial_channel",
    "scan_serial_ports",
    ScanSerialFn
);

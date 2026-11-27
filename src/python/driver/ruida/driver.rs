//! The `RuidaDriver` PyO3 wrapper: rpascript jobs and live commands
//! over USB, UDP or TCP, exposed through the uniform session surface
//! plus the protocol-specific motion and memory operations.

use std::sync::Arc;
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::driver::Driver;
use crate::driver::ruida::driver::{RuidaConfig, RuidaDriver};
use crate::driver::ruida::transport::TransportKind;
use crate::python::driver::base::PyDriver;
use crate::python::driver::events::PyEvents;
use crate::python::driver::grbl::driver::{get_f64, get_str};
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

fn build_config(dict: Option<&Bound<'_, PyDict>>) -> PyResult<RuidaConfig> {
    let mut config = RuidaConfig::default();
    let Some(dict) = dict else {
        return Ok(config);
    };
    if let Some(v) = get_str(dict, "host").ok().flatten() {
        if !v.is_empty() {
            config.host = Some(v);
        }
    }
    if let Some(v) = get_str(dict, "usb_device").ok().flatten() {
        if !v.is_empty() {
            config.usb_device = Some(v);
        }
    }
    if let Ok(Some(value)) = dict.get_item("port") {
        if let Ok(port) = value.extract::<u64>() {
            config.port = port as u16;
        }
    }
    if let Some(v) = get_str(dict, "protocol").ok().flatten() {
        config.protocol = if v.eq_ignore_ascii_case("tcp") {
            TransportKind::Tcp
        } else {
            TransportKind::Udp
        };
    }
    if let Some(v) = get_str(dict, "magic_number").ok().flatten() {
        config.magic = parse_magic(&v)?;
    }
    if let Ok(Some(value)) = dict.get_item("protect") {
        if let Ok(flag) = value.extract::<bool>() {
            config.protect = flag;
        }
    }
    if let Some(v) = get_f64(dict, "timeout")? {
        config.timeout = Duration::from_millis((v * 1000.0) as u64);
    }
    if let Some(v) = get_f64(dict, "query_interval")? {
        config.query_interval = Duration::from_secs_f64(v.max(0.05));
    }
    if let Ok(Some(value)) = dict.get_item("ping_retries") {
        if let Ok(retries) = value.extract::<u32>() {
            config.ping_retries = retries;
        }
    }
    if let Some(v) = get_f64(dict, "ping_delay")? {
        config.ping_delay = Duration::from_secs_f64(v.max(0.0));
    }
    Ok(config)
}

fn parse_magic(raw: &str) -> PyResult<u8> {
    let raw = raw.trim();
    let value = if let Some(hex) =
        raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).ok()
    } else {
        raw.parse::<u8>().ok()
    };
    value.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "magic number must be between 0x00 and 0xFF: {raw:?}"
        ))
    })
}

/// A direct connection to a Ruida laser controller.
///
/// All async methods return asyncio futures; a typed `DriverEvent`
/// is delivered to `event_callback(event)` on the event loop thread.
/// `run()` and `run_raw()` take rpascript text; the machine-code
/// encoder produces it from Ops.
#[gen_stub_pyclass(module = "raygeo.driver.ruida")]
#[pyclass(name = "RuidaDriver", module = "raygeo.driver.ruida", extends = PyDriver)]
pub struct PyRuidaDriver {
    core: Arc<RuidaDriver>,
}

impl PyRuidaDriver {
    fn initializer(
        config: RuidaConfig,
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let events = Arc::new(PyEvents::new(event_callback));
        let core = RuidaDriver::new(config, events.clone());
        let base = crate::python::driver::base::PyDriver::new(
            core.clone(),
            events.clone(),
        );
        Ok(PyClassInitializer::from(base).add_subclass(Self { core }))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyRuidaDriver {
    #[new]
    #[gen_stub(override_return_type(type_repr = "RuidaDriver"))]
    #[pyo3(signature = (config, dialect=None, event_callback=None))]
    fn new(
        #[gen_stub(override_type(
            type_repr = "typing.Optional[dict]",
            imports = ("typing",)
        ))]
        config: Option<Bound<'_, PyAny>>,

        #[gen_stub(override_type(
            type_repr = "typing.Optional[dict]",
            imports = ("typing",)
        ))]
        dialect: Option<Bound<'_, PyDict>>,

        #[gen_stub(override_type(
            type_repr = "typing.Optional[collections.abc.Callable[[core.DriverEvent], builtins.object]]",
            imports = ("builtins", "collections.abc", "typing", "raygeo.driver.core")
        ))]
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let _ = dialect;
        let config =
            crate::python::driver::config::config_dict(config.as_ref())?;
        let config = build_config(config.as_ref())?;
        Self::initializer(config, event_callback)
    }

    /// Move the XY axes to the current origin.
    fn home_xy<'py>(&self, py: Python<'py>) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.home_xy().await?;
            Ok(())
        })
        .expect("home_xy future")
    }

    /// Home the Z axis through the controller's homing routine.
    fn home_z<'py>(&self, py: Python<'py>) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.home_z().await?;
            Ok(())
        })
        .expect("home_z future")
    }

    /// Home the U axis.
    fn home_u<'py>(&self, py: Python<'py>) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.home_u().await?;
            Ok(())
        })
        .expect("home_u future")
    }

    /// Run the controller's Z auto-focus routine.
    fn focus_z<'py>(&self, py: Python<'py>) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.focus_z().await?;
            Ok(())
        })
        .expect("focus_z future")
    }

    /// Stop the job and home XY, like the panel's reset key.
    fn reset<'py>(&self, py: Python<'py>) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.reset().await?;
            Ok(())
        })
        .expect("reset future")
    }

    /// Jog to an absolute XY position in millimetres.
    fn jog_xy_to<'py>(
        &self,
        py: Python<'py>,
        x: f64,
        y: f64,
    ) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.jog_xy_to(x, y).await?;
            Ok(())
        })
        .expect("jog_xy_to future")
    }

    /// Jog to an absolute X position in millimetres.
    fn jog_x_to<'py>(&self, py: Python<'py>, x: f64) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.jog_x_to(x).await?;
            Ok(())
        })
        .expect("jog_x_to future")
    }

    /// Jog to an absolute Y position in millimetres.
    fn jog_y_to<'py>(&self, py: Python<'py>, y: f64) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.jog_y_to(y).await?;
            Ok(())
        })
        .expect("jog_y_to future")
    }

    /// Jog to an absolute Z position in millimetres.
    fn jog_z_to<'py>(&self, py: Python<'py>, z: f64) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.jog_z_to(z).await?;
            Ok(())
        })
        .expect("jog_z_to future")
    }

    /// Jog to an absolute U position in millimetres.
    fn jog_u_to<'py>(&self, py: Python<'py>, u: f64) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.jog_u_to(u).await?;
            Ok(())
        })
        .expect("jog_u_to future")
    }

    /// Jog XY relatively in millimetres.
    #[pyo3(signature = (x=None, y=None))]
    fn jog_xy_rel<'py>(
        &self,
        py: Python<'py>,
        x: Option<f64>,
        y: Option<f64>,
    ) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.jog_xy_rel(x, y).await?;
            Ok(())
        })
        .expect("jog_xy_rel future")
    }

    /// Jog one axis relatively in millimetres.
    fn jog_axis_rel<'py>(
        &self,
        py: Python<'py>,
        axis: String,
        delta: f64,
    ) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            let axis = axis
                .trim()
                .chars()
                .next()
                .unwrap_or('x')
                .to_ascii_lowercase();
            core.jog_axis_rel(axis, delta).await?;
            Ok(())
        })
        .expect("jog_axis_rel future")
    }

    /// Set the jog speeds in mm/s.
    fn set_jog_speeds(&self, xy: f64, z: f64, u: f64) {
        self.core.set_jog_speeds(xy, z, u);
    }

    /// Send one raw rpascript line.
    fn execute<'py>(&self, py: Python<'py>, line: String) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.execute(&line).await?;
            Ok(())
        })
        .expect("execute future")
    }

    /// Read one controller memory address; returns ``(address,
    /// data)`` pairs.
    fn get_setting<'py>(
        &self,
        py: Python<'py>,
        address: u16,
    ) -> PyAwait<'py, Py<PyList>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            let replies = core.get_setting(address).await?;
            Python::attach(|py| {
                let list = PyList::empty(py);
                for reply in replies {
                    let data = PyList::new(py, reply.data)?;
                    list.append((reply.address, data))?;
                }
                Ok(list.unbind())
            })
        })
        .expect("get_setting future")
    }

    /// Write one controller memory address. Blocked while protect
    /// mode is on.
    fn set_setting<'py>(
        &self,
        py: Python<'py>,
        address: u16,
        a: u64,
        b: u64,
    ) -> PyAwait<'py, ()> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.set_setting(address, a, b).await?;
            Ok(())
        })
        .expect("set_setting future")
    }

    /// Block or allow ``SET_SETTING`` commands.
    fn set_protect(&self, enabled: bool) {
        self.core.set_protect(enabled);
    }

    /// Whether protect mode blocks ``SET_SETTING``.
    #[getter]
    fn protect_enabled(&self) -> bool {
        self.core.protect_enabled()
    }

    /// The last decoded controller status.
    #[getter]
    fn machine_status<'py>(&self, py: Python<'py>) -> PyResult<Py<PyDict>> {
        let status = self.core.machine_status();
        let dict = PyDict::new(py);
        dict.set_item("running", status.running)?;
        dict.set_item("paused", status.paused)?;
        dict.set_item("moving", status.moving)?;
        let position = PyList::empty(py);
        for value in status.position {
            match value {
                Some(value) => position.append(value)?,
                None => position.append(py.None())?,
            }
        }
        dict.set_item("position", position)?;
        dict.set_item("card_id", status.card_id)?;
        dict.set_item(
            "bed_size",
            status.bed_size.map(|value| value.unwrap_or(0.0)),
        )?;
        dict.set_item("mainboard_version", status.mainboard_version)?;
        dict.set_item("features", status.features)?;
        Ok(dict.unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "RuidaDriver(resource_uri={:?}, connected={}, job_running={})",
            self.core.resource_uri(),
            self.core.is_connected(),
            self.core.is_job_running(),
        )
    }
}

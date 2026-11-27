//! The `raygeo.driver.driver` PyO3 module: the `Driver` base class.
//!
//! Every protocol driver class is a real Python subclass of
//! [`PyDriver`] (PyO3 `extends`), so the uniform surface is declared
//! exactly once here, inherited through the MRO, and visible once in
//! the generated stubs.  The base dispatches to the driver object
//! stored as `Arc<dyn Driver>`; protocol subclasses keep their
//! concrete `Arc` for their extras.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::driver::core::driver::{CancelMode, Driver, DriverInfo};

use crate::python::driver::epilog::driver::PyEpilogDriver;
use crate::python::driver::grbl::driver::{
    PyGrblBluetoothDriver, PyGrblNetworkDriver, PyGrblSerialDriver,
    PyGrblTelnetDriver,
};
use crate::python::driver::grbl::simple::PyGrblSerialSimpleDriver;
use crate::python::driver::marlin::driver::PyMarlinSerialDriver;
use crate::python::driver::octoprint::driver::PyOctoPrintDriver;
use crate::python::driver::ruida::driver::PyRuidaDriver;
use crate::python::driver::simulator::PyNoDeviceDriver;
use crate::python::driver::smoothie::driver::PySmoothieDriver;
use crate::python::driver::xtool_d1::driver::PyXtoolNetworkDriver;
#[cfg(feature = "m2")]
use crate::python::driver::xtool_m2::driver::PyXtoolM2Driver;

pub(crate) mod info;
use crate::python::driver::core::types::DeviceState;
use crate::python::driver::events::PyEvents;
use crate::python::driver::pyfuture::{
    spawn_future, spawn_future_with_cancel, PyAwait,
};

/// The base class of every raygeo driver.
///
/// Declares the uniform surface once: connection lifecycle, job
/// streaming, machine control, probing and settings.  Operations a
/// protocol cannot perform raise `NotImplementedError`; the method
/// is still present on every driver.
#[gen_stub_pyclass]
#[pyclass(name = "Driver", module = "raygeo.driver", subclass)]
pub struct PyDriver {
    pub(crate) core: Arc<dyn Driver>,
    pub(crate) events: Arc<PyEvents>,
    resource: Arc<Mutex<Option<crate::driver::registry::ResourceGuard>>>,
}

impl PyDriver {
    /// Build the base layer for a concrete driver.
    #[allow(dead_code)]
    pub(crate) fn new(core: Arc<dyn Driver>, events: Arc<PyEvents>) -> Self {
        Self {
            core,
            events,
            resource: Arc::new(Mutex::new(None)),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDriver {
    /// Open the connection and keep it up until `disconnect`.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        self.events.capture_loop(py)?;
        {
            let mut slot = self.resource.lock().unwrap();
            if slot.is_none() {
                *slot = Some(crate::driver::registry::ResourceGuard::acquire(
                    self.core.resource_uri().as_deref(),
                    None,
                )?);
            }
        }
        let core = self.core.clone();
        let resource = self.resource.clone();
        spawn_future(py, async move {
            match Driver::connect(core).await {
                Ok(()) => Ok(()),
                Err(err) => {
                    resource.lock().unwrap().take();
                    Err(err.into())
                }
            }
        })
    }

    /// Stop all background tasks and close the connection.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        let events = self.events.clone();
        spawn_future(py, async move {
            Driver::cleanup(&*core).await;
            events.quiesce();
            Ok(())
        })
    }

    /// Replace the command templates.
    fn update_dialect(&self, dialect: Bound<'_, PyDict>) -> PyResult<()> {
        self.core.update_dialect(
            crate::python::driver::grbl::driver::parse_dialect(Some(&dialect))?,
        );
        Ok(())
    }

    /// Send one interactive command and return its response lines.
    fn execute_interactive_command<'py>(
        &self,
        py: Python<'py>,
        command: String,
    ) -> PyResult<PyAwait<'py, Vec<String>>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Ok(Driver::execute_interactive_command(&*core, &command).await?)
        })
    }

    /// Send one command and return its response lines.
    fn execute_command<'py>(
        &self,
        py: Python<'py>,
        command: String,
    ) -> PyResult<PyAwait<'py, Vec<String>>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Ok(Driver::execute_command(&*core, &command).await?)
        })
    }

    /// Stream a G-code job.
    #[pyo3(signature = (gcode, op_line_map=None, op_estimates=None, progress_callback=None))]
    fn run<'py>(
        &self,
        py: Python<'py>,
        gcode: String,
        op_line_map: Option<HashMap<usize, i64>>,
        op_estimates: Option<Vec<f64>>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[collections.abc.Callable[[builtins.int], builtins.object]]",
            imports = ("typing", "collections.abc")
        ))]
        progress_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyAwait<'py, ()>> {
        self.events.capture_loop(py)?;
        let core = self.core.clone();
        let events = self.events.clone();
        let op_map = op_line_map.unwrap_or_default();
        let estimates = op_estimates.unwrap_or_default();
        let cancel_core = core.clone();
        spawn_future_with_cancel(
            py,
            async move {
                events.set_progress_callback(progress_callback);
                Driver::run(core, &gcode, &op_map, &estimates).await?;
                Ok(())
            },
            move || {
                crate::python::driver::runtime::handle().spawn(async move {
                    let _ = Driver::cancel(&*cancel_core, CancelMode::Graceful)
                        .await;
                });
            },
        )
    }

    /// Execute pre-rendered machine code.
    fn run_raw<'py>(
        &self,
        py: Python<'py>,
        machine_code: String,
    ) -> PyResult<PyAwait<'py, ()>> {
        self.events.capture_loop(py)?;
        let core = self.core.clone();
        let cancel_core = core.clone();
        spawn_future_with_cancel(
            py,
            async move {
                Driver::run_raw(core, &machine_code).await?;
                Ok(())
            },
            move || {
                crate::python::driver::runtime::handle().spawn(async move {
                    let _ = Driver::cancel(&*cancel_core, CancelMode::Graceful)
                        .await;
                });
            },
        )
    }

    /// Cancel the running job.
    #[pyo3(signature = (emergency=false))]
    fn cancel<'py>(
        &self,
        py: Python<'py>,
        emergency: bool,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            let mode = if emergency {
                CancelMode::Immediate
            } else {
                CancelMode::Graceful
            };
            Driver::cancel(&*core, mode).await?;
            Ok(())
        })
    }

    /// Pause or resume the running job.
    #[pyo3(signature = (hold=true))]
    fn set_hold<'py>(
        &self,
        py: Python<'py>,
        hold: bool,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Driver::set_hold(&*core, hold).await?;
            Ok(())
        })
    }

    /// Home all axes or the given axes.
    #[pyo3(signature = (axes=None, active_wcs=None))]
    fn home<'py>(
        &self,
        py: Python<'py>,
        axes: Option<Vec<String>>,
        active_wcs: Option<String>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        let active_wcs = active_wcs.unwrap_or_else(|| "G54".to_string());
        spawn_future(py, async move {
            Driver::home(&*core, axes, &active_wcs).await?;
            Ok(())
        })
    }

    /// Absolute move in the machine's current coordinate frame.
    #[pyo3(signature = (speed, x, y, z=None))]
    fn move_to<'py>(
        &self,
        py: Python<'py>,
        speed: f64,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Driver::move_to(&*core, speed, x, y, z).await?;
            Ok(())
        })
    }

    /// Relative jog by per-axis distances.
    #[pyo3(signature = (speed, deltas=None))]
    fn jog<'py>(
        &self,
        py: Python<'py>,
        speed: f64,
        deltas: Option<Vec<(String, f64)>>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        let deltas = deltas.unwrap_or_default();
        spawn_future(py, async move {
            Driver::jog(&*core, speed, &deltas).await?;
            Ok(())
        })
    }

    /// Select a tool.
    fn select_tool<'py>(
        &self,
        py: Python<'py>,
        tool_number: i64,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Driver::select_tool(&*core, tool_number).await?;
            Ok(())
        })
    }

    /// Set the laser/spindle power; `None` disables the tool.
    #[pyo3(signature = (power=None))]
    fn set_power<'py>(
        &self,
        py: Python<'py>,
        power: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Driver::set_power(&*core, power).await?;
            Ok(())
        })
    }

    /// Set the laser power used while focusing.
    #[pyo3(signature = (power=None))]
    fn set_focus_power<'py>(
        &self,
        py: Python<'py>,
        power: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Driver::set_focus_power(&*core, power).await?;
            Ok(())
        })
    }

    /// Clear a device alarm.
    fn clear_alarm<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Driver::clear_alarm(&*core).await?;
            Ok(())
        })
    }

    /// Write a work-coordinate-system offset.
    #[pyo3(signature = (wcs_slot, x, y, z=None))]
    fn set_wcs_offset<'py>(
        &self,
        py: Python<'py>,
        wcs_slot: String,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Driver::set_wcs_offset(&*core, &wcs_slot, x, y, z).await?;
            Ok(())
        })
    }

    /// Read the device's firmware settings.
    fn read_settings<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, Vec<(String, String)>>> {
        let core = self.core.clone();
        spawn_future(
            py,
            async move { Ok(Driver::read_settings(&*core).await?) },
        )
    }

    /// Write one firmware setting.
    fn write_setting<'py>(
        &self,
        py: Python<'py>,
        key: String,
        value: String,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Driver::write_setting(&*core, &key, &value).await?;
            Ok(())
        })
    }

    /// Read the work-coordinate-system offsets.
    fn read_wcs_offsets<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, crate::driver::core::driver::WcsOffsets>> {
        let core = self.core.clone();
        spawn_future(
            py,
            async move { Ok(Driver::read_wcs_offsets(&*core).await?) },
        )
    }

    /// Read the G-code parser state.
    fn read_parser_state<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, Option<String>>> {
        let core = self.core.clone();
        spawn_future(
            py,
            async move { Ok(Driver::read_parser_state(&*core).await?) },
        )
    }

    /// Run a probe cycle; `None` = no trigger reported.
    fn run_probe_cycle<'py>(
        &self,
        py: Python<'py>,
        axis_letter: String,
        max_travel: f64,
        feed_rate: f64,
    ) -> PyResult<PyAwait<'py, Option<(f64, f64, f64)>>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            Ok(Driver::run_probe_cycle(
                &*core,
                &axis_letter,
                max_travel,
                feed_rate,
            )
            .await?)
        })
    }

    /// Detect the device's unit system (`"metric"` or `"imperial"`),
    /// or `None` when unknown.
    fn detect_unit_system<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, Option<String>>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            let unit = Driver::detect_unit_system(&*core).await?;
            Ok(unit.map(|unit| {
                match unit {
                    crate::driver::core::types::UnitSystem::Metric => "metric",
                    crate::driver::core::types::UnitSystem::Imperial => {
                        "imperial"
                    }
                }
                .to_string()
            }))
        })
    }

    /// The driver's typed identity and capabilities.
    #[getter]
    fn info(&self) -> crate::python::driver::base::info::DriverInfo {
        crate::python::driver::base::info::DriverInfo::new(
            self.core.info().clone(),
        )
    }

    /// Current snapshot of the device state.
    #[getter]
    fn state(&self) -> DeviceState {
        DeviceState::new(Driver::state(&*self.core))
    }

    /// True while a job is running or held.
    #[getter]
    fn job_running(&self) -> bool {
        Driver::is_job_running(&*self.core)
    }

    /// Bytes currently occupying the device's RX buffer.
    #[getter]
    fn buffer_count(&self) -> usize {
        Driver::buffer_count(&*self.core)
    }

    /// The device's RX buffer size in bytes, or 0 when unknown.
    #[getter]
    fn rx_buffer_size(&self) -> usize {
        Driver::rx_buffer_size(&*self.core)
    }

    /// Commands sent but not yet acknowledged.
    #[getter]
    fn pending_commands(&self) -> Vec<String> {
        Driver::pending_commands(&*self.core)
    }

    /// Identifier of the physical resource, or `None`.
    #[getter]
    fn resource_uri(&self) -> Option<String> {
        Driver::resource_uri(&*self.core)
    }
}

impl Drop for PyDriver {
    fn drop(&mut self) {
        self.events.close();
        self.core.abort_background_tasks_now();
    }
}

/// The natural move target for a host-held driver list.
#[allow(dead_code)]
pub(crate) fn info_of(driver: &dyn Driver) -> DriverInfo {
    driver.info().clone()
}

/// Look up a built-in driver's typed metadata by class name.
#[gen_stub_pyfunction(module = "raygeo.driver")]
#[pyfunction]
fn driver_info(name: &str) -> PyResult<info::DriverInfo> {
    let kind = crate::driver::registry::DriverKind::from_name(name)
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(name.to_string())
        })?;
    Ok(info::DriverInfo::new(kind.info().clone()))
}

static DRIVERS: LazyLock<Mutex<Option<Py<PyAny>>>> =
    LazyLock::new(|| Mutex::new(None));

fn builtin_drivers(py: Python<'_>) -> Vec<Py<PyAny>> {
    vec![
        py.get_type::<PyGrblSerialDriver>().into_any().unbind(),
        py.get_type::<PyGrblTelnetDriver>().into_any().unbind(),
        py.get_type::<PyGrblBluetoothDriver>().into_any().unbind(),
        py.get_type::<PyGrblNetworkDriver>().into_any().unbind(),
        py.get_type::<PyGrblSerialSimpleDriver>()
            .into_any()
            .unbind(),
        py.get_type::<PyMarlinSerialDriver>().into_any().unbind(),
        py.get_type::<PySmoothieDriver>().into_any().unbind(),
        #[cfg(feature = "m2")]
        py.get_type::<PyXtoolM2Driver>().into_any().unbind(),
        py.get_type::<PyXtoolNetworkDriver>().into_any().unbind(),
        py.get_type::<PyOctoPrintDriver>().into_any().unbind(),
        py.get_type::<PyEpilogDriver>().into_any().unbind(),
        py.get_type::<PyRuidaDriver>().into_any().unbind(),
        py.get_type::<PyNoDeviceDriver>().into_any().unbind(),
    ]
}

/// Resolve a persisted driver name.
#[gen_stub_pyfunction(module = "raygeo.driver")]
#[gen_stub(override_return_type(type_repr = "type[Driver]"))]
#[pyfunction]
#[pyo3(signature = (name, default=None))]
fn get_driver_cls(
    py: Python<'_>,
    name: &str,
    default: Option<Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    let drivers = DRIVERS.lock().unwrap().clone().ok_or_else(|| {
        pyo3::exceptions::PyRuntimeError::new_err(
            "the driver list is not initialized",
        )
    })?;
    for driver in drivers.bind(py).try_iter()? {
        let driver = driver?;
        let class_name: String = driver.getattr("__name__")?.extract()?;
        if class_name == name {
            return Ok(driver.unbind());
        }
    }
    match default {
        Some(default) => Ok(default),
        None => Err(pyo3::exceptions::PyKeyError::new_err(name.to_string())),
    }
}

/// Register an addon driver class.
#[gen_stub_pyfunction(module = "raygeo.driver")]
#[pyfunction]
fn register_driver(
    py: Python<'_>,
    #[gen_stub(override_type(type_repr = "type[Driver]"))] driver: Py<PyAny>,
) -> PyResult<()> {
    let drivers = DRIVERS.lock().unwrap().clone().ok_or_else(|| {
        pyo3::exceptions::PyRuntimeError::new_err(
            "the driver list is not initialized",
        )
    })?;
    drivers.bind(py).call_method1("append", (driver,))?;
    Ok(())
}

pub(crate) const MODULE_DOC: &str = "\
The common driver base class and its typed metadata.

Every protocol driver is a subclass of Driver: the uniform surface \
(connection lifecycle, job streaming, machine control, probing and \
settings) is declared here once, with strict types, and inherited by \
every driver class. Operations a protocol cannot perform raise \
NotImplementedError.
";

pyo3_stub_gen::module_doc!("raygeo.driver", "{}", MODULE_DOC);

pub(crate) fn populate(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = module.py();
    module.setattr("__doc__", MODULE_DOC)?;
    module.setattr("__name__", "raygeo.driver")?;
    module.setattr("__package__", "raygeo")?;
    module.add("__all__", vec!["Driver", "DriverInfo", "driver_info"])?;
    module.add_class::<PyDriver>()?;
    module.add_class::<info::DriverInfo>()?;
    module.add_class::<info::SetupField>()?;
    module.add_function(wrap_pyfunction!(driver_info, module)?)?;
    let drivers = pyo3::types::PyList::new(py, builtin_drivers(py))?;
    module.add("drivers", &drivers)?;
    *DRIVERS.lock().unwrap() = Some(drivers.into_any().unbind());
    module.add_function(wrap_pyfunction!(get_driver_cls, module)?)?;
    module.add_function(wrap_pyfunction!(register_driver, module)?)?;
    Ok(())
}

/// Stub type of the `drivers` module list: classes, not instances.
struct DriverClassList;

impl pyo3_stub_gen::PyStubType for DriverClassList {
    fn type_output() -> pyo3_stub_gen::TypeInfo {
        let mut import = std::collections::HashSet::new();
        import.insert("builtins".into());
        pyo3_stub_gen::TypeInfo {
            name: "builtins.list[type[Driver]]".to_string(),
            source_module: None,
            import,
            type_refs: Default::default(),
        }
    }
}

pyo3_stub_gen::module_variable!("raygeo.driver", "drivers", DriverClassList);

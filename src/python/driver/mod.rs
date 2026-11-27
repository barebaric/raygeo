//! Python bindings for the machine-driver layer (`raygeo.driver`).

pub(crate) mod base;
pub(crate) mod census;
pub(crate) mod config;
pub(crate) mod core;
pub(crate) mod epilog;
pub(crate) mod events;
pub(crate) mod grbl;
pub(crate) mod marlin;
pub(crate) mod octoprint;
pub(crate) mod pyfuture;
pub(crate) mod remote;
pub(crate) mod ruida;
pub(crate) mod runtime;
pub(crate) mod simulator;
pub(crate) mod smoothie;
pub(crate) mod xtool_d1;
#[cfg(feature = "m2")]
pub(crate) mod xtool_m2;

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

/// UTC timestamp in RFC3339-ish form, without pulling in a date
/// crate (Howard Hinnant's civil-from-days algorithm).
fn utc_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = {
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (y + i64::from(m <= 2), m as u32, d as u32)
    };
    format!(
        "{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{millis:03}Z",
        h = rem / 3600,
        mi = rem % 3600 / 60,
        s = rem % 60,
    )
}

struct LoggerState {
    filter: env_filter::Filter,
    file: Option<std::fs::File>,
}

static LOGGER: Mutex<Option<LoggerState>> = Mutex::new(None);

struct BridgeLogger;

impl log_crate::Log for BridgeLogger {
    fn enabled(&self, metadata: &log_crate::Metadata) -> bool {
        LOGGER
            .lock()
            .map(|guard| {
                guard.as_ref().is_some_and(|s| s.filter.enabled(metadata))
            })
            .unwrap_or(false)
    }

    fn log(&self, record: &log_crate::Record) {
        let line = format!(
            "[{} {:<5} {}] {}\n",
            utc_timestamp(),
            record.level(),
            record.target(),
            record.args()
        );
        let guard = LOGGER.lock();
        let mut guard = match guard {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some(state) = guard.as_mut() else {
            return;
        };
        if !state.filter.matches(record) {
            return;
        }
        match state.file.as_mut() {
            Some(file) => {
                let _ = file.write_all(line.as_bytes());
            }
            None => {
                let _ = std::io::stderr().write_all(line.as_bytes());
            }
        }
    }

    fn flush(&self) {
        let guard = LOGGER.lock();
        let mut guard = match guard {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(state) = guard.as_mut() {
            if let Some(file) = state.file.as_mut() {
                let _ = file.flush();
            }
        }
        let _ = std::io::stderr().flush();
    }
}

/// Initialize the Rust-side logger (`log` crate) so session
/// diagnostics reach stderr or, when *path* is given, a dedicated
/// log file (opened in append mode).  The filter defaults to
/// ``warn`` and can be overridden with an ``env_logger`` filter
/// string argument or via ``RUST_LOG`` (which always wins).  Calling
/// again re-targets output and replaces the filter.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (filter = None, path = None))]
fn init_logging(filter: Option<&str>, path: Option<&str>) {
    let spec = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| filter.unwrap_or("warn").to_string());
    let mut filter_builder = env_filter::Builder::new();
    filter_builder.parse(&spec);
    let filter = filter_builder.build();
    log_crate::set_max_level(filter.filter());
    let file = path.and_then(|p| {
        OpenOptions::new().create(true).append(true).open(p).ok()
    });
    // Installing the global logger succeeds only on the first call;
    // later calls just retarget the shared state below.
    let _ = log_crate::set_boxed_logger(Box::new(BridgeLogger));
    *LOGGER.lock().unwrap() = Some(LoggerState { filter, file });
}

/// Tear down the shared tokio runtime (see
/// [`runtime::shutdown`]).  Called via `atexit`, before CPython
/// finalizes the interpreter.
#[pyfunction]
fn _shutdown_runtime() {
    runtime::shutdown();
}

fn register_atexit_hook(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let atexit = py.import("atexit")?;
    let hook = wrap_pyfunction!(_shutdown_runtime, m)?;
    atexit.call_method1("register", (hook,))?;
    Ok(())
}

/// Register the root-level logging helpers on the `raygeo` module.
pub(crate) fn register_root(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init_logging, m)?)?;
    register_atexit_hook(m)?;
    Ok(())
}

/// Register the `raygeo.driver` layer and all protocol submodules.
pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let driver_mod = PyModule::new(py, "driver")?;
    base::populate(&driver_mod)?;
    config::register(&driver_mod)?;
    core::register(&driver_mod)?;
    grbl::register(&driver_mod)?;
    marlin::register(&driver_mod)?;
    simulator::register(&driver_mod)?;
    smoothie::register(&driver_mod)?;
    octoprint::register(&driver_mod)?;
    epilog::register(&driver_mod)?;
    xtool_d1::register(&driver_mod)?;
    #[cfg(feature = "m2")]
    xtool_m2::register(&driver_mod)?;
    remote::register(&driver_mod)?;
    ruida::register(&driver_mod)?;
    // Mounted at raygeo.raygeo.driver. The public raygeo.driver is the
    // Python package on disk, which forwards attribute access here;
    // taking over the public name in sys.modules would hide that
    // package and break imports such as raygeo.driver.grbl.
    driver_mod.setattr("__name__", "raygeo.raygeo.driver")?;
    driver_mod.setattr("__package__", "raygeo.raygeo")?;
    parent.add_submodule(&driver_mod)?;
    Ok(())
}

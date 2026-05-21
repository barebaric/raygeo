use pyo3::prelude::*;

use raygeo_core::ops::optimize::{optimize_travel, ProgressCallback};

struct PyProgress<'py> {
    cb: Option<&'py Bound<'py, PyAny>>,
}

impl<'py> ProgressCallback for PyProgress<'py> {
    fn report(&self, progress: f64, message: &str) {
        if let Some(cb) = self.cb {
            let _ = cb.call1((progress, message));
        }
    }

    fn is_cancelled(&self) -> bool {
        if let Some(cb) = self.cb {
            if let Ok(result) = cb.call_method0("is_cancelled") {
                if let Ok(cancelled) = result.extract::<bool>() {
                    return cancelled;
                }
            }
        }
        false
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let algo_mod = PyModule::new(m.py(), "algo")?;
    algo_mod.add_function(wrap_pyfunction!(
        optimize_travel_py,
        algo_mod.clone()
    )?)?;
    m.add_submodule(&algo_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.algo", &algo_mod)?;
    sys_modules.set_item("raygeo.ops.algo.optimize", &algo_mod)?;

    Ok(())
}

#[pyfunction(name = "optimize_travel")]
#[pyo3(signature = (ops, allow_flip=true, preserve_first=false, preserve_order=Vec::new(), progress_cb=None))]
fn optimize_travel_py(
    ops: &mut crate::ops::container::PyOps,
    allow_flip: bool,
    preserve_first: bool,
    preserve_order: Vec<String>,
    progress_cb: Option<&Bound<'_, PyAny>>,
) -> PyResult<()> {
    let py_progress = PyProgress { cb: progress_cb };
    optimize_travel(
        &mut ops.inner,
        allow_flip,
        preserve_first,
        preserve_order,
        Some(&py_progress),
    );
    Ok(())
}

use pyo3::prelude::*;

use crate::ops::transform::optimize::{optimize_travel, ProgressCallback};

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

pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let opt_mod = PyModule::new(transform_mod.py(), "optimize")?;
    opt_mod
        .add_function(wrap_pyfunction!(optimize_travel_py, opt_mod.clone())?)?;
    transform_mod.add_submodule(&opt_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.optimize", &opt_mod)?;

    Ok(())
}

#[pyfunction(name = "optimize_travel")]
#[pyo3(signature = (ops, allow_flip=true, preserve_first=false, preserve_order=Vec::new(), progress_cb=None))]
fn optimize_travel_py(
    ops: &mut crate::python::ops::PyOps,
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

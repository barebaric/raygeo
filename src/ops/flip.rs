use pyo3::prelude::*;

use super::container::PyOps;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "flip")?;
    m.add_function(wrap_pyfunction!(py_flip_ops, &m)?)?;
    parent.add_submodule(&m)?;

    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.flip", &m)?;

    Ok(())
}

#[pyfunction]
fn py_flip_ops(ops: &PyOps) -> PyOps {
    PyOps {
        inner: ops.inner.flip_ops(),
    }
}

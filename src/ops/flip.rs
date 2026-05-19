use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use super::container::PyOps;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "flip")?;
    m.add_function(wrap_pyfunction!(py_flip_ops, &m)?)?;
    parent.add_submodule(&m)?;

    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.flip", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(python = r#"
    def flip_ops(ops: Ops) -> Ops:
        """Flip the ops."""
"#, module = "raygeo.ops")]
#[pyfunction]
fn py_flip_ops(ops: &PyOps) -> PyOps {
    PyOps {
        inner: ops.inner.flip_ops(),
    }
}

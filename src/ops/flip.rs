use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use super::container::PyOps;

/// Register the ``raygeo.ops.flip`` submodule with the parent module.
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
        """Reverse the order and direction of all commands in the ops.

        This mirrors the operations so that the sequence is
        reversed, which is useful for traversing a path backwards.

        :param ops: The operations to flip.
        :returns: A new Ops with reversed order and direction.
        """
"#, module = "raygeo.ops.flip")]
#[pyfunction]
fn py_flip_ops(ops: &PyOps) -> PyOps {
    PyOps {
        inner: ops.inner.flip_ops(),
    }
}

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::transform::link::{
    find_pass_entry, find_pass_exit, link_passes, LinkStrategy,
};
use crate::python::ops::PyOps;

pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let link_mod = PyModule::new(transform_mod.py(), "link")?;
    link_mod
        .add_function(wrap_pyfunction!(link_passes_py, link_mod.clone())?)?;
    link_mod.add_function(wrap_pyfunction!(
        find_pass_entry_py,
        link_mod.clone()
    )?)?;
    link_mod
        .add_function(wrap_pyfunction!(find_pass_exit_py, link_mod.clone())?)?;
    link_mod.add_class::<PyLinkStrategy>()?;
    transform_mod.add_submodule(&link_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.link", &link_mod)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.ops import Ops

    def link_passes(
        passes: list[ops.Ops],
        safe_z: float,
        strategy: str | LinkStrategy,
    ) -> ops.Ops:
        """Join ordered machining passes into a single Ops sequence.

        The first pass is emitted as-is; subsequent passes are prefixed
        with travel moves according to *strategy*:

        * ``"retract"`` / ``LinkStrategy.RETRACT`` — retract to
          *safe_z*, move XY at that height, then descend to the next
          pass start Z.
        * ``"stay_down"`` / ``LinkStrategy.STAY_DOWN`` — move directly
          from the previous pass end to the next pass start without
          retracting.

        :param passes: Ordered list of :class:`~raygeo.ops.Ops` passes.
        :param safe_z: Z height for retract moves (mm).
        :param strategy: Linking strategy.
        :returns: A single :class:`~raygeo.ops.Ops` container.
        :complexity: O(n) where n = total commands across all passes
        """
    "#,
    module = "raygeo.ops.transform.link"
)]
#[pyfunction(name = "link_passes")]
#[pyo3(signature = (passes, safe_z, strategy))]
fn link_passes_py(
    passes: &Bound<'_, PyList>,
    safe_z: f64,
    strategy: &str,
) -> PyResult<PyOps> {
    let strategy_enum = match strategy {
        "retract" | "Retract" => LinkStrategy::Retract,
        "stay_down" | "StayDown" => LinkStrategy::StayDown,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!(
                    "unknown LinkStrategy: '{strategy}'; expected 'retract' or 'stay_down'"
                ),
            ));
        }
    };

    let mut opss = Vec::with_capacity(passes.len());
    for item in passes.iter() {
        let py_ops: PyRef<'_, PyOps> = item.extract()?;
        opss.push(py_ops.inner.clone());
    }

    let ops = link_passes(&opss, safe_z, strategy_enum);
    Ok(PyOps { inner: ops })
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.ops import Ops

    def find_pass_entry(
        ops: ops.Ops,
    ) -> tuple[float, float, float] | None:
        """Find the entry point of an Ops sequence.

        Scans for the first travel (MoveTo) endpoint, falling back to
        the first moving command endpoint.

        :param ops: An :class:`~raygeo.ops.Ops` container.
        :returns: ``(x, y, z)`` or ``None`` if no moving commands exist.
        :complexity: O(n) where n = number of commands
        """
    "#,
    module = "raygeo.ops.transform.link"
)]
#[pyfunction(name = "find_pass_entry")]
fn find_pass_entry_py(
    ops: &Bound<'_, PyOps>,
) -> PyResult<Option<(f64, f64, f64)>> {
    let inner = ops.borrow();
    match find_pass_entry(&inner.inner) {
        Some(pt) => Ok(Some((pt.x, pt.y, pt.z))),
        None => Ok(None),
    }
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.ops import Ops

    def find_pass_exit(
        ops: ops.Ops,
    ) -> tuple[float, float, float] | None:
        """Find the exit point of an Ops sequence.

        Scans backwards for the last moving command endpoint.

        :param ops: An :class:`~raygeo.ops.Ops` container.
        :returns: ``(x, y, z)`` or ``None`` if no moving commands exist.
        :complexity: O(n) where n = number of commands
        """
    "#,
    module = "raygeo.ops.transform.link"
)]
#[pyfunction(name = "find_pass_exit")]
fn find_pass_exit_py(
    ops: &Bound<'_, PyOps>,
) -> PyResult<Option<(f64, f64, f64)>> {
    let inner = ops.borrow();
    match find_pass_exit(&inner.inner) {
        Some(pt) => Ok(Some((pt.x, pt.y, pt.z))),
        None => Ok(None),
    }
}

#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.link",
    name = "LinkStrategy",
    skip_from_py_object
)]
struct PyLinkStrategy;

#[gen_stub_pymethods]
#[pymethods]
impl PyLinkStrategy {
    #[classattr]
    const RETRACT: &'static str = "retract";
    #[classattr]
    const STAY_DOWN: &'static str = "stay_down";
}

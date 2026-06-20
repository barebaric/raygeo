use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::polyline::{
    find_pass_entry, find_pass_exit, link_passes, polyline_to_ops, LinkStrategy,
};
use crate::python::ops::PyOps;

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let poly_mod = PyModule::new(m.py(), "polyline")?;
    poly_mod.add_function(wrap_pyfunction!(
        polyline_to_ops_py,
        poly_mod.clone()
    )?)?;
    poly_mod
        .add_function(wrap_pyfunction!(link_passes_py, poly_mod.clone())?)?;
    poly_mod.add_function(wrap_pyfunction!(
        find_pass_entry_py,
        poly_mod.clone()
    )?)?;
    poly_mod
        .add_function(wrap_pyfunction!(find_pass_exit_py, poly_mod.clone())?)?;
    poly_mod.add_class::<PyLinkStrategy>()?;
    m.add_submodule(&poly_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.polyline", &poly_mod)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.ops import Ops

    def polyline_to_ops(
        points: list[tuple[float, float, float]],
        move_first: bool = True,
    ) -> ops.Ops:
        """Convert a 3-D polyline into an Ops command sequence.

        When *move_first* is ``True`` the first point is emitted as a
        MoveTo and subsequent points as LineTo.  When *move_first* is
        ``False`` every point is emitted as a LineTo (useful for
        appending a polyline to an in-progress cut).

        :param points: List of ``(x, y, z)`` tuples.
        :param move_first: Whether to emit the first point as a MoveTo.
        :returns: An :class:`~raygeo.ops.Ops` container.
        :complexity: O(n) where n = number of points
        """
    "#,
    module = "raygeo.ops.polyline"
)]
#[pyfunction(name = "polyline_to_ops")]
#[pyo3(signature = (points, move_first=true))]
fn polyline_to_ops_py(
    points: &Bound<'_, PyList>,
    move_first: bool,
) -> PyResult<PyOps> {
    let mut pts = Vec::with_capacity(points.len());
    for item in points.iter() {
        let t: (f64, f64, f64) = item.extract()?;
        pts.push(crate::types::Point3D::new(t.0, t.1, t.2));
    }
    let ops = polyline_to_ops(&pts, move_first);
    Ok(PyOps { inner: ops })
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
    module = "raygeo.ops.polyline"
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
    module = "raygeo.ops.polyline"
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
    module = "raygeo.ops.polyline"
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
    module = "raygeo.ops.polyline",
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

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::assembly::polyline::polyline_to_ops;
use crate::python::ops::PyOps;

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let poly_mod = PyModule::new(m.py(), "polyline")?;
    poly_mod.add_function(wrap_pyfunction!(
        polyline_to_ops_py,
        poly_mod.clone()
    )?)?;
    m.add_submodule(&poly_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.polyline", &poly_mod)?;

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
    module = "raygeo.ops.assembly.polyline"
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

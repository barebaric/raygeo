//! Python bindings for `raygeo.ops.material.grid`.

use numpy::IntoPyArray;
use numpy::PyReadonlyArray2;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::material::grid::compute_power_uvs;
use crate::ops::material::spec::GridSpec;
pyo3_stub_gen::module_doc!("raygeo.ops.material.grid", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Burn-grid helpers: per-vertex power UVs for a stock mesh.
";

/// Map a mesh's vertex positions onto a burn power grid as normalized
/// UVs.
///
/// ``power_uv = ((xy - origin) * px_per_mm) / size_px``, so a vertex
/// at the grid's origin corner maps to ``(0, 0)`` and one at the far
/// corner maps to ``(1, 1)``. Returns an ``(N, 2)`` float32 array
/// index-aligned with the input ``positions`` (``(N, 3)``).
#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing
    import collections.abc

    def compute_power_uvs(
        positions: numpy.typing.NDArray[numpy.float32],
        origin_mm: tuple[float, float],
        px_per_mm: tuple[float, float],
        size_px: tuple[int, int],
    ) -> numpy.typing.NDArray[numpy.float32]:
        """Map vertex positions onto a burn power grid as UVs.

        :param positions: Flat mesh vertex positions, shape ``(N, 3)``.
        :param origin_mm: World-mm coordinate of the grid's ``(0, 0)``
            pixel corner.
        :param px_per_mm: Grid density in pixels per millimetre.
        :param size_px: Grid size in pixels ``(width, height)``.
        :returns: ``(N, 2)`` power UVs index-aligned with *positions*.
        """
    "#,
    module = "raygeo.ops.material.grid"
)]
#[pyfunction(name = "compute_power_uvs")]
#[pyo3(signature = (positions, origin_mm, px_per_mm, size_px))]
fn compute_power_uvs_py<'py>(
    py: Python<'py>,
    positions: PyReadonlyArray2<'_, f32>,
    origin_mm: (f64, f64),
    px_per_mm: (f64, f64),
    size_px: (usize, usize),
) -> PyResult<Bound<'py, PyAny>> {
    let arr = positions.as_array();
    if arr.shape()[1] != 3 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "positions must be an (N, 3) array",
        ));
    }
    let n = arr.shape()[0];
    let flat: Vec<f32> = arr.iter().copied().collect();
    let grid = GridSpec {
        origin_mm,
        px_per_mm,
        size_px,
    };
    let uvs = compute_power_uvs(&flat, &grid);
    let rows = n as isize;
    let array = uvs.into_pyarray(py).into_any();
    array.call_method1("reshape", (rows, 2))
}

pub(crate) fn register(mat_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = mat_mod.py();
    let m = PyModule::new(py, "grid")?;
    m.setattr("__doc__", MODULE_DOC)?;
    register_functions!(m, compute_power_uvs_py,);

    mat_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.material.grid", &m)?;

    Ok(())
}

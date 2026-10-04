//! PyO3 binding for
//! [`MeshCorrectionSpec`](crate::ops::transform::mesh_correction::MeshCorrectionSpec).

use numpy::{IntoPyArray, PyArray2, PyArrayMethods, PyReadonlyArray2};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::mesh_correction::MeshCorrectionSpec as CoreSpec;

/// Register the `MeshCorrectionSpec` class on the `mesh_correction`
/// submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let mesh_mod = PyModule::new(transform_mod.py(), "mesh_correction")?;
    mesh_mod.add_class::<MeshCorrectionSpec>()?;
    transform_mod.add_submodule(&mesh_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.mesh_correction", &mesh_mod)?;

    Ok(())
}

/// A probed bed height map for
/// :meth:`raygeo.ops.Ops.mesh_correction`.
///
/// Construct with
/// ``MeshCorrectionSpec(x0, y0, dx, dy, heights, z_offset)``; invalid
/// grids raise ``ValueError`` at construction.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.mesh_correction",
    name = "MeshCorrectionSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct MeshCorrectionSpec {
    /// X coordinate of the first grid column.
    #[pyo3(get)]
    pub x0: f64,
    /// Y coordinate of the first grid row.
    #[pyo3(get)]
    pub y0: f64,
    /// Spacing between grid columns along X.
    #[pyo3(get)]
    pub dx: f64,
    /// Spacing between grid rows along Y.
    #[pyo3(get)]
    pub dy: f64,
    /// Constant Z added on top of every sampled height.
    #[pyo3(get)]
    pub z_offset: f64,
    pub(crate) heights: Vec<f64>,
    pub(crate) nx: usize,
    pub(crate) ny: usize,
}

impl MeshCorrectionSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreSpec {
        CoreSpec {
            x0: self.x0,
            y0: self.y0,
            dx: self.dx,
            dy: self.dy,
            heights: self.heights,
            nx: self.nx,
            ny: self.ny,
            z_offset: self.z_offset,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MeshCorrectionSpec {
    /// :param x0: X coordinate of the first grid column.
    /// :param y0: Y coordinate of the first grid row.
    /// :param dx: Grid column spacing in millimetres.
    /// :param dy: Grid row spacing in millimetres.
    /// :param heights: 2-D float64 array of shape ``(ny, nx)``;
    ///     ``heights[j, i]`` is the surface height at
    ///     ``(x0 + i*dx, y0 + j*dy)``.
    /// :param z_offset: Constant Z added on top of every sampled
    ///     height.
    #[new]
    #[pyo3(signature = (x0, y0, dx, dy, heights, z_offset = 0.0))]
    fn new(
        x0: f64,
        y0: f64,
        dx: f64,
        dy: f64,
        heights: PyReadonlyArray2<f64>,
        z_offset: f64,
    ) -> PyResult<Self> {
        let array = heights.as_array();
        let (ny, nx) = (array.shape()[0], array.shape()[1]);
        let spec = CoreSpec::validated(CoreSpec {
            x0,
            y0,
            dx,
            dy,
            heights: array.iter().copied().collect(),
            nx,
            ny,
            z_offset,
        })
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(MeshCorrectionSpec {
            x0: spec.x0,
            y0: spec.y0,
            dx: spec.dx,
            dy: spec.dy,
            z_offset: spec.z_offset,
            heights: spec.heights,
            nx: spec.nx,
            ny: spec.ny,
        })
    }

    /// The probed heights as a ``(ny, nx)`` float64 array.
    #[getter]
    fn heights<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        self.heights
            .clone()
            .into_pyarray(py)
            .reshape((self.ny, self.nx))
            .expect("heights length matches grid shape")
    }
}

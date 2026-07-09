use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::ops::cut::search::PyToolPose;
use crate::python::ops::PyOps;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    assembly_mod.add_class::<PyAssemblyResult>()?;
    Ok(())
}

/// Universal return type for every assembly-level generator.
///
/// Returned by assemblers such as ``generate_helix``,
/// ``generate_toroidal_clear``, ``generate_slot``, and all other
/// assembly-level motion functions.  Contains the generated ``Ops``
/// sequence, the set of polygons that this operation clears, and the
/// tool pose at the start and end of the path.
#[gen_stub_pyclass(module = "raygeo.ops.assembly.result")]
#[pyclass(
    name = "AssemblyResult",
    skip_from_py_object,
    module = "raygeo.ops.assembly.result"
)]
#[derive(Clone, Debug)]
pub struct PyAssemblyResult {
    pub inner: crate::ops::assembly::result::AssemblyResult,
    #[pyo3(get)]
    pub ops: PyOps,
    #[pyo3(get)]
    pub cleared_polygons: Vec<Vec<(f64, f64)>>,
    #[pyo3(get)]
    pub start: PyToolPose,
    #[pyo3(get)]
    pub end: PyToolPose,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAssemblyResult {
    #[new]
    fn __new__() -> Self {
        let inner = crate::ops::assembly::result::AssemblyResult {
            ops: crate::ops::Ops::new(),
            cleared_polygons: vec![],
            start: crate::ops::cut::ToolPose {
                pos: crate::types::Point3D::ZERO,
                heading: 0.0,
            },
            end: crate::ops::cut::ToolPose {
                pos: crate::types::Point3D::ZERO,
                heading: 0.0,
            },
        };
        PyAssemblyResult::from_inner(inner)
    }

    fn __repr__(&self) -> String {
        let n_ops = self.ops.inner.len();
        let n_polys = self.cleared_polygons.len();
        format!(
            "AssemblyResult(ops={n_ops} commands, cleared_polygons={n_polys}, \
             start=({sx:.3},{sy:.3},{sz:.3}), end=({ex:.3},{ey:.3},{ez:.3}))",
            sx = self.start.pos.0,
            sy = self.start.pos.1,
            sz = self.start.pos.2,
            ex = self.end.pos.0,
            ey = self.end.pos.1,
            ez = self.end.pos.2,
        )
    }
}

impl PyAssemblyResult {
    pub fn from_inner(
        inner: crate::ops::assembly::result::AssemblyResult,
    ) -> Self {
        let cleared_polys: Vec<Vec<(f64, f64)>> = inner
            .cleared_polygons
            .iter()
            .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
            .collect();

        PyAssemblyResult {
            ops: PyOps {
                inner: inner.ops.clone(),
            },
            cleared_polygons: cleared_polys,
            start: PyToolPose {
                pos: (inner.start.pos.x, inner.start.pos.y, inner.start.pos.z),
                heading: inner.start.heading,
            },
            end: PyToolPose {
                pos: (inner.end.pos.x, inner.end.pos.y, inner.end.pos.z),
                heading: inner.end.heading,
            },
            inner,
        }
    }
}

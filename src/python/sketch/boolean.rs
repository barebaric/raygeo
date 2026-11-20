//! Python bindings for boolean region collection and operations.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::types::Point;
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::boolean::{
    BooleanOp as CoreBooleanOp, BooleanRegion as CoreBooleanRegion,
    BooleanSolid as CoreBooleanSolid,
};

/// Boolean operation kind.
#[pyo3_stub_gen::derive::gen_stub_pyclass_complex_enum]
#[pyclass(
    eq,
    eq_int,
    from_py_object,
    module = "raygeo.sketch.boolean",
    name = "BooleanOp"
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyBooleanOp {
    #[pyo3(name = "UNION")]
    Union,
    #[pyo3(name = "DIFFERENCE")]
    Difference,
    #[pyo3(name = "INTERSECTION")]
    Intersection,
    #[pyo3(name = "EXCLUDE")]
    Exclude,
}

/// One closed candidate region.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.boolean",
    name = "BooleanRegion",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyBooleanRegion {
    #[pyo3(get, set)]
    pub rings: Vec<Vec<(f64, f64)>>,
    #[pyo3(get, set)]
    pub entity_ids: Vec<u64>,
    #[pyo3(get, set)]
    pub z: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyBooleanRegion {
    #[new]
    #[pyo3(signature = (rings, entity_ids, z=0))]
    fn new(
        rings: Vec<Vec<(f64, f64)>>,
        entity_ids: Vec<u64>,
        z: usize,
    ) -> Self {
        PyBooleanRegion {
            rings,
            entity_ids,
            z,
        }
    }
}

/// One boolean result: outer ring plus holes.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.boolean",
    name = "BooleanSolid",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyBooleanSolid {
    #[pyo3(get)]
    pub outer: Vec<(f64, f64)>,
    #[pyo3(get)]
    pub holes: Vec<Vec<(f64, f64)>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyBooleanSolid {
    #[new]
    fn new(
        outer: Vec<(f64, f64)>,
        holes: Option<Vec<Vec<(f64, f64)>>>,
    ) -> Self {
        PyBooleanSolid {
            outer,
            holes: holes.unwrap_or_default(),
        }
    }
}

fn from_core_region(region: CoreBooleanRegion) -> PyBooleanRegion {
    PyBooleanRegion {
        rings: region
            .rings
            .into_iter()
            .map(|ring| ring.into_iter().map(|p| (p.x, p.y)).collect())
            .collect(),
        entity_ids: region.entity_ids,
        z: region.z,
    }
}

fn from_core_solid(solid: CoreBooleanSolid) -> PyBooleanSolid {
    PyBooleanSolid {
        outer: solid.outer.into_iter().map(|p| (p.x, p.y)).collect(),
        holes: solid
            .holes
            .into_iter()
            .map(|ring| ring.into_iter().map(|p| (p.x, p.y)).collect())
            .collect(),
    }
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction(module = "raygeo.sketch.boolean")]
#[pyfunction]
#[pyo3(signature = (
    registry,
    entity_ids,
    coincident_pairs = Vec::new(),
    tolerance = 0.02
))]
fn build_boolean_regions(
    registry: PyRef<'_, PyEntityRegistry>,
    entity_ids: Vec<u64>,
    coincident_pairs: Vec<(u64, u64)>,
    tolerance: f64,
) -> Option<Vec<PyBooleanRegion>> {
    crate::sketch::boolean::build_boolean_regions(
        &registry.inner,
        &entity_ids,
        &coincident_pairs,
        tolerance,
    )
    .map(|regions| regions.into_iter().map(from_core_region).collect())
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction(module = "raygeo.sketch.boolean")]
#[pyfunction]
#[pyo3(signature = (op, regions, tolerance = 0.02))]
fn apply_boolean(
    op: PyBooleanOp,
    regions: Vec<PyRef<'_, PyBooleanRegion>>,
    tolerance: f64,
) -> Option<Vec<PyBooleanSolid>> {
    let op = match op {
        PyBooleanOp::Union => CoreBooleanOp::Union,
        PyBooleanOp::Difference => CoreBooleanOp::Difference,
        PyBooleanOp::Intersection => CoreBooleanOp::Intersection,
        PyBooleanOp::Exclude => CoreBooleanOp::Exclude,
    };
    let regions: Vec<CoreBooleanRegion> = regions
        .into_iter()
        .map(|region| CoreBooleanRegion {
            rings: region
                .rings
                .iter()
                .map(|ring| {
                    ring.iter().map(|&(x, y)| Point::new(x, y)).collect()
                })
                .collect(),
            entity_ids: region.entity_ids.clone(),
            z: region.z,
        })
        .collect();
    crate::sketch::boolean::apply_boolean(op, &regions, tolerance)
        .map(|solids| solids.into_iter().map(from_core_solid).collect())
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let boolean_mod = PyModule::new(py, "boolean")?;
    boolean_mod.setattr(
        "__doc__",
        "Boolean operations over sketch geometry: region collection and \
         Clipper-backed result grouping.",
    )?;
    boolean_mod.add_class::<PyBooleanOp>()?;
    boolean_mod.add_class::<PyBooleanRegion>()?;
    boolean_mod.add_class::<PyBooleanSolid>()?;
    boolean_mod
        .add_function(wrap_pyfunction!(build_boolean_regions, &boolean_mod)?)?;
    boolean_mod.add_function(wrap_pyfunction!(apply_boolean, &boolean_mod)?)?;
    m.add_submodule(&boolean_mod)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.boolean", &boolean_mod)?;
    Ok(())
}

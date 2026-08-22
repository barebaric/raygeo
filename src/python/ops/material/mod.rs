//! Python bindings for `raygeo.ops.material`.

use numpy::PyReadonlyArray2;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::compressed_array::CompressedArray;
use crate::geo::types::{Point, Point3D, Polygon};
use crate::mesh::solid::SolidMesh;
use crate::ops::material::spec::{GridSpec, MaterialResponse};
use crate::ops::material::MaterialEffect;
use crate::python::compressed_array::PyCompressedArray;

pub(crate) mod fold;
pub(crate) mod grid;
pub(crate) mod spec;
pub(crate) mod state;

pyo3_stub_gen::module_doc!("raygeo.ops.material", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Material-effect folding: classifying what operations removed.

Assemblers emit MaterialEffects alongside their Ops — a unified
description of the material they remove, for CNC and laser alike.
fold_effects() aggregates the effects of many operations against one
stock into an immutable MaterialState snapshot: through-cut voids,
the burn surface map, provenance, and escalation signals for geometry
the current profiles cannot represent exactly.
";

fn tuples_to_polygons(tuples: &[Vec<(f64, f64)>]) -> Vec<Polygon> {
    tuples
        .iter()
        .map(|ring| ring.iter().map(|p| Point::new(p.0, p.1)).collect())
        .collect()
}

fn polygons_to_tuples(polygons: &[Polygon]) -> Vec<Vec<(f64, f64)>> {
    polygons
        .iter()
        .map(|ring| ring.iter().map(|p| (p.x, p.y)).collect())
        .collect()
}

/// A vector material effect: polygons removed over a Z interval.
///
/// Z values use the toolpath convention (stock surface at ``z = 0``,
/// bottom at ``z = -thickness``): ``z_from=None`` means open to the
/// surface, ``z_to=None`` means through the bottom.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.material", name = "VectorEffect")]
pub struct PyVectorEffect {
    pub(crate) inner: MaterialEffect,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyVectorEffect {
    #[new]
    #[pyo3(signature = (polygons, z_from = None, z_to = None))]
    fn new(
        polygons: Vec<Vec<(f64, f64)>>,
        z_from: Option<f64>,
        z_to: Option<f64>,
    ) -> Self {
        PyVectorEffect {
            inner: MaterialEffect::Vector {
                polygons: tuples_to_polygons(&polygons),
                z_from,
                z_to,
            },
        }
    }

    /// Footprint polygons in workpiece-local mm.
    #[getter]
    fn polygons(&self) -> Vec<Vec<(f64, f64)>> {
        match &self.inner {
            MaterialEffect::Vector { polygons, .. } => {
                polygons_to_tuples(polygons)
            }
            _ => Vec::new(),
        }
    }

    /// Top of the removed interval; ``None`` = open to the surface.
    #[getter]
    fn z_from(&self) -> Option<f64> {
        match &self.inner {
            MaterialEffect::Vector { z_from, .. } => *z_from,
            _ => None,
        }
    }

    /// Bottom of the removed interval; ``None`` = through the bottom.
    #[getter]
    fn z_to(&self) -> Option<f64> {
        match &self.inner {
            MaterialEffect::Vector { z_to, .. } => *z_to,
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MaterialEffect::Vector {
                polygons,
                z_from,
                z_to,
            } => format!(
                "VectorEffect(polygons={}, z_from={:?}, z_to={:?})",
                polygons.len(),
                z_from,
                z_to
            ),
            _ => "VectorEffect(invalid)".to_string(),
        }
    }
}

/// A raster material effect: an F32 fluence map plus its grid placement.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.material", name = "RasterEffect")]
pub struct PyRasterEffect {
    pub(crate) inner: MaterialEffect,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyRasterEffect {
    #[new]
    #[pyo3(signature = (fluence, origin_mm, px_per_mm, cut_fluence_threshold = None))]
    fn new(
        fluence: PyReadonlyArray2<f32>,
        origin_mm: (f64, f64),
        px_per_mm: (f64, f64),
        cut_fluence_threshold: Option<f32>,
    ) -> PyResult<Self> {
        let array = fluence.as_array();
        let (h, w) = (array.shape()[0], array.shape()[1]);
        let data: Vec<f32> = match array.as_slice() {
            Some(slice) => slice.to_vec(),
            None => array.iter().copied().collect(),
        };
        if w == 0 || h == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "fluence map must be non-empty",
            ));
        }
        let grid = GridSpec {
            origin_mm,
            px_per_mm,
            size_px: (w, h),
        };
        let response = MaterialResponse {
            cut_fluence_threshold,
        };
        Ok(PyRasterEffect {
            inner: MaterialEffect::Raster {
                fluence: CompressedArray::from_vec_f32_with_shape(
                    data,
                    vec![h, w],
                ),
                grid,
                response,
            },
        })
    }

    /// The fluence map as a compressed F32 array.
    #[getter]
    fn fluence(&self) -> PyResult<PyCompressedArray> {
        match &self.inner {
            MaterialEffect::Raster { fluence, .. } => {
                Ok(PyCompressedArray::from_inner(fluence.clone()))
            }
            _ => Err(pyo3::exceptions::PyValueError::new_err(
                "not a raster effect",
            )),
        }
    }

    /// World-mm origin of the grid's (0, 0) pixel corner.
    #[getter]
    fn origin_mm(&self) -> (f64, f64) {
        match &self.inner {
            MaterialEffect::Raster { grid, .. } => grid.origin_mm,
            _ => (0.0, 0.0),
        }
    }

    /// Grid density in pixels per millimetre ``(x, y)``.
    #[getter]
    fn px_per_mm(&self) -> (f64, f64) {
        match &self.inner {
            MaterialEffect::Raster { grid, .. } => grid.px_per_mm,
            _ => (0.0, 0.0),
        }
    }

    /// Raster fluence (J/cm²) at or above which the material is cut
    /// through.
    #[getter]
    fn cut_fluence_threshold(&self) -> Option<f32> {
        match &self.inner {
            MaterialEffect::Raster { response, .. } => {
                response.cut_fluence_threshold
            }
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MaterialEffect::Raster { grid, .. } => format!(
                "RasterEffect(size_px={:?}, origin_mm={:?}, px_per_mm={:?})",
                grid.size_px, grid.origin_mm, grid.px_per_mm
            ),
            _ => "RasterEffect(invalid)".to_string(),
        }
    }
}

/// A volume material effect: closed solids to be removed.
///
/// No assembler emits these yet; the variant exists so future 3D
/// assemblers join the same fold without a wire-format change.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.material", name = "VolumeEffect")]
pub struct PyVolumeEffect {
    pub(crate) inner: MaterialEffect,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyVolumeEffect {
    #[new]
    fn new(
        positions: Vec<(f64, f64, f64)>,
        triangles: Vec<(u32, u32, u32)>,
    ) -> Self {
        let solids = vec![SolidMesh::new(
            positions
                .iter()
                .map(|p| Point3D::new(p.0, p.1, p.2))
                .collect(),
            triangles.iter().map(|t| [t.0, t.1, t.2]).collect(),
        )];
        PyVolumeEffect {
            inner: MaterialEffect::Volume { solids },
        }
    }

    /// Vertex positions of the first solid (world mm).
    #[getter]
    fn positions(&self) -> Vec<(f64, f64, f64)> {
        match &self.inner {
            MaterialEffect::Volume { solids } => solids
                .first()
                .map(|s| s.positions.iter().map(|p| (p.x, p.y, p.z)).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Triangle indices of the first solid.
    #[getter]
    fn triangles(&self) -> Vec<(u32, u32, u32)> {
        match &self.inner {
            MaterialEffect::Volume { solids } => solids
                .first()
                .map(|s| {
                    s.triangles.iter().map(|t| (t[0], t[1], t[2])).collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MaterialEffect::Volume { solids } => format!(
                "VolumeEffect(solids={}, triangles={})",
                solids.len(),
                solids.first().map(|s| s.triangles.len()).unwrap_or(0)
            ),
            _ => "VolumeEffect(invalid)".to_string(),
        }
    }
}

/// Convert a core effect into its Python wrapper object.
pub(crate) fn effect_to_py(
    py: Python<'_>,
    effect: &MaterialEffect,
) -> PyResult<Py<PyAny>> {
    match effect {
        MaterialEffect::Vector { .. } => Ok(Py::new(
            py,
            PyVectorEffect {
                inner: effect.clone(),
            },
        )?
        .into_any()),
        MaterialEffect::Raster { .. } => Ok(Py::new(
            py,
            PyRasterEffect {
                inner: effect.clone(),
            },
        )?
        .into_any()),
        MaterialEffect::Volume { .. } => Ok(Py::new(
            py,
            PyVolumeEffect {
                inner: effect.clone(),
            },
        )?
        .into_any()),
    }
}

/// Extract a core effect from one of the effect wrapper objects.
pub(crate) fn effect_from_py(
    obj: &Bound<'_, PyAny>,
) -> PyResult<MaterialEffect> {
    if let Ok(v) = obj.extract::<PyRef<'_, PyVectorEffect>>() {
        return Ok(v.inner.clone());
    }
    if let Ok(r) = obj.extract::<PyRef<'_, PyRasterEffect>>() {
        return Ok(r.inner.clone());
    }
    if let Ok(v) = obj.extract::<PyRef<'_, PyVolumeEffect>>() {
        return Ok(v.inner.clone());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "effects must be VectorEffect, RasterEffect, or VolumeEffect",
    ))
}

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let m = PyModule::new(py, "material")?;
    m.setattr("__doc__", MODULE_DOC)?;
    m.add_class::<PyVectorEffect>()?;
    m.add_class::<PyRasterEffect>()?;
    m.add_class::<PyVolumeEffect>()?;

    spec::register(&m)?;
    state::register(&m)?;
    fold::register(&m)?;
    grid::register(&m)?;

    ops_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.material", &m)?;

    Ok(())
}

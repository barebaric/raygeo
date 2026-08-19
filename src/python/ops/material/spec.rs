//! Python bindings for `raygeo.ops.material.spec`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::matrix::Matrix as CoreMatrix;
use crate::geo::types::Polygon;
use crate::ops::material::spec::{
    FoldEntry, GridBudget, MaterialFoldSpec, StockShape,
};
use crate::python::geo::matrix::Matrix;

use super::{effect_from_py, polygons_to_tuples, tuples_to_polygons};

pyo3_stub_gen::module_doc!("raygeo.ops.material.spec", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Fold input types: the stock, the entries, and the grid budget.
";

/// A prismatic stock: 2D outline extruded over a thickness.
///
/// Z convention: top surface at ``z = 0``, bottom at
/// ``z = -thickness``.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.material.spec",
    name = "PrismaticStock",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPrismaticStock {
    pub(crate) polygons: Vec<Polygon>,
    pub(crate) thickness: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPrismaticStock {
    #[new]
    fn new(polygons: Vec<Vec<(f64, f64)>>, thickness: f64) -> Self {
        PyPrismaticStock {
            polygons: tuples_to_polygons(&polygons),
            thickness,
        }
    }

    /// Stock outline polygons in world mm.
    #[getter]
    fn polygons(&self) -> Vec<Vec<(f64, f64)>> {
        polygons_to_tuples(&self.polygons)
    }

    /// Stock thickness in mm.
    #[getter]
    fn thickness(&self) -> f64 {
        self.thickness
    }
}

/// Resolution budget for stock-grid outputs.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.material.spec",
    name = "GridBudget",
    from_py_object
)]
#[derive(Clone)]
pub struct PyGridBudget {
    pub(crate) px_per_mm: f64,
    pub(crate) max_px: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyGridBudget {
    #[new]
    #[pyo3(signature = (px_per_mm = 50.0, max_px = 8192))]
    fn new(px_per_mm: f64, max_px: usize) -> Self {
        PyGridBudget { px_per_mm, max_px }
    }

    /// Requested grid density in pixels per millimetre.
    #[getter]
    fn px_per_mm(&self) -> f64 {
        self.px_per_mm
    }

    /// Per-side pixel cap; ``px_per_mm`` is scaled down to fit.
    #[getter]
    fn max_px(&self) -> usize {
        self.max_px
    }
}

/// One compute node's contribution to a fold.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.material.spec",
    name = "FoldEntry",
    from_py_object
)]
#[derive(Clone)]
pub struct PyFoldEntry {
    pub(crate) source_key: String,
    pub(crate) placement: CoreMatrix,
    pub(crate) effects: Vec<Py<PyAny>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyFoldEntry {
    #[new]
    fn new(
        source_key: String,
        placement: &Matrix,
        effects: Vec<Py<PyAny>>,
    ) -> Self {
        PyFoldEntry {
            source_key,
            placement: placement.inner,
            effects,
        }
    }

    /// Node key of the source (for provenance).
    #[getter]
    fn source_key(&self) -> String {
        self.source_key.clone()
    }

    /// Workpiece-local to world-mm placement of the effects.
    #[getter]
    fn placement(&self) -> Matrix {
        Matrix {
            inner: self.placement,
        }
    }

    /// The effects this entry contributes.
    #[getter]
    fn effects(&self) -> Vec<Py<PyAny>> {
        self.effects.clone()
    }
}

impl PyFoldEntry {
    fn to_core(&self, py: Python<'_>) -> PyResult<FoldEntry> {
        let mut effects = Vec::with_capacity(self.effects.len());
        for obj in &self.effects {
            let bound = obj.bind(py);
            effects.push(effect_from_py(bound)?);
        }
        Ok(FoldEntry {
            source_key: self.source_key.clone(),
            placement: self.placement,
            effects,
        })
    }
}

/// Full input to :func:`fold_effects`.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.material.spec", name = "MaterialFoldSpec")]
pub struct PyMaterialFoldSpec {
    pub(crate) stock: PyPrismaticStock,
    pub(crate) entries: Vec<PyFoldEntry>,
    pub(crate) grid: PyGridBudget,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyMaterialFoldSpec {
    #[new]
    #[pyo3(signature = (stock, entries, grid = None))]
    fn new(
        stock: PyPrismaticStock,
        entries: Vec<PyFoldEntry>,
        grid: Option<PyGridBudget>,
    ) -> Self {
        PyMaterialFoldSpec {
            stock,
            entries,
            grid: grid.unwrap_or(PyGridBudget {
                px_per_mm: 50.0,
                max_px: 8192,
            }),
        }
    }

    /// The stock to fold against.
    #[getter]
    fn stock(&self) -> PyPrismaticStock {
        PyPrismaticStock {
            polygons: self.stock.polygons.clone(),
            thickness: self.stock.thickness,
        }
    }

    /// Effect-bearing entries, in any order.
    #[getter]
    fn entries(&self) -> Vec<PyFoldEntry> {
        self.entries
            .iter()
            .map(|e| PyFoldEntry {
                source_key: e.source_key.clone(),
                placement: e.placement,
                effects: e.effects.clone(),
            })
            .collect()
    }

    /// Grid budget for raster outputs.
    #[getter]
    fn grid(&self) -> PyGridBudget {
        PyGridBudget {
            px_per_mm: self.grid.px_per_mm,
            max_px: self.grid.max_px,
        }
    }
}

impl PyMaterialFoldSpec {
    pub(crate) fn to_core(&self, py: Python<'_>) -> PyResult<MaterialFoldSpec> {
        let mut entries = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            entries.push(entry.to_core(py)?);
        }
        Ok(MaterialFoldSpec {
            stock: StockShape::Prismatic {
                polygons: self.stock.polygons.clone(),
                thickness: self.stock.thickness,
            },
            entries,
            grid: GridBudget {
                px_per_mm: self.grid.px_per_mm,
                max_px: self.grid.max_px,
            },
        })
    }
}

/// Grid placement of raster outputs in world mm.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.material.spec", name = "GridSpec")]
pub struct PyGridSpec {
    pub(crate) origin_mm: (f64, f64),
    pub(crate) px_per_mm: (f64, f64),
    pub(crate) size_px: (usize, usize),
}

#[gen_stub_pymethods]
#[pymethods]
impl PyGridSpec {
    /// World-mm origin of the grid's (0, 0) pixel corner.
    #[getter]
    fn origin_mm(&self) -> (f64, f64) {
        self.origin_mm
    }

    /// Grid density in pixels per millimetre ``(x, y)``.
    #[getter]
    fn px_per_mm(&self) -> (f64, f64) {
        self.px_per_mm
    }

    /// Grid size in pixels ``(width, height)``.
    #[getter]
    fn size_px(&self) -> (usize, usize) {
        self.size_px
    }
}

pub(crate) fn register(mat_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = mat_mod.py();
    let m = PyModule::new(py, "spec")?;
    m.setattr("__doc__", MODULE_DOC)?;
    m.add_class::<PyPrismaticStock>()?;
    m.add_class::<PyGridBudget>()?;
    m.add_class::<PyFoldEntry>()?;
    m.add_class::<PyMaterialFoldSpec>()?;
    m.add_class::<PyGridSpec>()?;

    mat_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.material.spec", &m)?;

    Ok(())
}

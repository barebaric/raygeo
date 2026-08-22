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

/// A cylindrical (rotary) stock, folded in unrolled space.
///
/// The axial coordinate maps to world x in ``[0, length]``; the
/// circumference (arc length) maps to world y in
/// ``[-pi * diameter / 2, pi * diameter / 2]``.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.material.spec",
    name = "CylinderStock",
    from_py_object
)]
#[derive(Clone)]
pub struct PyCylinderStock {
    pub(crate) diameter: f64,
    pub(crate) length: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCylinderStock {
    #[new]
    fn new(diameter: f64, length: f64) -> Self {
        PyCylinderStock { diameter, length }
    }

    /// Workpiece diameter in mm.
    #[getter]
    fn diameter(&self) -> f64 {
        self.diameter
    }

    /// Axial length in mm.
    #[getter]
    fn length(&self) -> f64 {
        self.length
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
    pub(crate) stock: PyStockShape,
    pub(crate) entries: Vec<PyFoldEntry>,
    pub(crate) grid: PyGridBudget,
    pub(crate) wavelength_nm: f64,
    pub(crate) max_power_watts: f64,
}

/// The stock variants a fold spec accepts.
pub(crate) enum PyStockShape {
    Prismatic(PyPrismaticStock),
    Cylinder(PyCylinderStock),
}

fn extract_stock(obj: &Bound<'_, PyAny>) -> PyResult<PyStockShape> {
    if let Ok(stock) = obj.extract::<PyPrismaticStock>() {
        return Ok(PyStockShape::Prismatic(stock));
    }
    if let Ok(stock) = obj.extract::<PyCylinderStock>() {
        return Ok(PyStockShape::Cylinder(stock));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "stock must be a PrismaticStock or CylinderStock",
    ))
}

#[gen_stub_pymethods]
#[pymethods]
impl PyMaterialFoldSpec {
    #[new]
    #[pyo3(signature = (stock, entries, grid = None, wavelength_nm = 0.0, max_power_watts = 0.0))]
    fn new(
        stock: &Bound<'_, PyAny>,
        entries: Vec<PyFoldEntry>,
        grid: Option<PyGridBudget>,
        wavelength_nm: f64,
        max_power_watts: f64,
    ) -> PyResult<Self> {
        Ok(PyMaterialFoldSpec {
            stock: extract_stock(stock)?,
            entries,
            grid: grid.unwrap_or(PyGridBudget {
                px_per_mm: 50.0,
                max_px: 8192,
            }),
            wavelength_nm,
            max_power_watts,
        })
    }

    /// The stock to fold against.
    #[getter]
    fn stock<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        match &self.stock {
            PyStockShape::Prismatic(s) => {
                let obj = s.clone().into_pyobject(py)?;
                Ok(obj.unbind().into_any())
            }
            PyStockShape::Cylinder(s) => {
                let obj = s.clone().into_pyobject(py)?;
                Ok(obj.unbind().into_any())
            }
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

    /// Emission wavelength in nm (0 = unconfigured).
    #[getter]
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    #[setter]
    fn set_wavelength_nm(&mut self, value: f64) {
        self.wavelength_nm = value;
    }

    /// Optical output power in watts at full power (0 = unconfigured).
    #[getter]
    fn max_power_watts(&self) -> f64 {
        self.max_power_watts
    }

    #[setter]
    fn set_max_power_watts(&mut self, value: f64) {
        self.max_power_watts = value;
    }
}

impl PyMaterialFoldSpec {
    pub(crate) fn to_core(&self, py: Python<'_>) -> PyResult<MaterialFoldSpec> {
        let mut entries = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            entries.push(entry.to_core(py)?);
        }
        let stock = match &self.stock {
            PyStockShape::Prismatic(s) => StockShape::Prismatic {
                polygons: s.polygons.clone(),
                thickness: s.thickness,
            },
            PyStockShape::Cylinder(s) => StockShape::Cylinder {
                diameter: s.diameter,
                length: s.length,
            },
        };
        Ok(MaterialFoldSpec {
            stock,
            entries,
            grid: GridBudget {
                px_per_mm: self.grid.px_per_mm,
                max_px: self.grid.max_px,
            },
            wavelength_nm: self.wavelength_nm,
            max_power_watts: self.max_power_watts,
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
    m.add_class::<PyCylinderStock>()?;
    m.add_class::<PyGridBudget>()?;
    m.add_class::<PyFoldEntry>()?;
    m.add_class::<PyMaterialFoldSpec>()?;
    m.add_class::<PyGridSpec>()?;

    mat_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.material.spec", &m)?;

    Ok(())
}

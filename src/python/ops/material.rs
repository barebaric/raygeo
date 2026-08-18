//! Python bindings for `raygeo.ops.material`.

use numpy::PyReadonlyArray2;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::compressed_array::CompressedArray;
use crate::geo::matrix::Matrix as CoreMatrix;
use crate::geo::types::{Point, Point3D, Polygon};
use crate::mesh::solid::SolidMesh;
use crate::ops::material::fold;
use crate::ops::material::spec::{
    FoldEntry, GridBudget, GridSpec, MaterialFoldSpec, MaterialResponse,
    StockShape,
};
use crate::ops::material::state::MaterialState;
use crate::ops::material::{FoldProfile, MaterialEffect};
use crate::python::compressed_array::PyCompressedArray;
use crate::python::geo::matrix::Matrix;

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

/// A raster material effect: an R8 power map plus its grid placement.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.material", name = "RasterEffect")]
pub struct PyRasterEffect {
    pub(crate) inner: MaterialEffect,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyRasterEffect {
    #[new]
    #[pyo3(signature = (power, origin_mm, px_per_mm, cut_power_threshold = None))]
    fn new(
        power: PyReadonlyArray2<u8>,
        origin_mm: (f64, f64),
        px_per_mm: (f64, f64),
        cut_power_threshold: Option<u8>,
    ) -> PyResult<Self> {
        let array = power.as_array();
        let (h, w) = (array.shape()[0], array.shape()[1]);
        let data: Vec<u8> = match array.as_slice() {
            Some(slice) => slice.to_vec(),
            None => array.iter().map(|v| *v).collect(),
        };
        if w == 0 || h == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "power map must be non-empty",
            ));
        }
        let grid = GridSpec {
            origin_mm,
            px_per_mm,
            size_px: (w, h),
        };
        let response = MaterialResponse {
            cut_power_threshold,
        };
        Ok(PyRasterEffect {
            inner: MaterialEffect::Raster {
                power: CompressedArray::from_vec_u8(data, vec![h, w]),
                grid,
                response,
            },
        })
    }

    /// The power map as a compressed R8 array.
    #[getter]
    fn power(&self) -> PyResult<PyCompressedArray> {
        match &self.inner {
            MaterialEffect::Raster { power, .. } => {
                Ok(PyCompressedArray::from_inner(power.clone()))
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

    /// Raster power at or above which the material is cut through.
    #[getter]
    fn cut_power_threshold(&self) -> Option<u8> {
        match &self.inner {
            MaterialEffect::Raster { response, .. } => {
                response.cut_power_threshold
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
fn effect_from_py(obj: &Bound<'_, PyAny>) -> PyResult<MaterialEffect> {
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

/// A prismatic stock: 2D outline extruded over a thickness.
///
/// Z convention: top surface at ``z = 0``, bottom at
/// ``z = -thickness``.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.material",
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
#[pyclass(module = "raygeo.ops.material", name = "GridBudget", from_py_object)]
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
#[pyclass(module = "raygeo.ops.material", name = "FoldEntry", from_py_object)]
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
    fn into_core(&self, py: Python<'_>) -> PyResult<FoldEntry> {
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
#[pyclass(module = "raygeo.ops.material", name = "MaterialFoldSpec")]
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
    fn into_core(&self, py: Python<'_>) -> PyResult<MaterialFoldSpec> {
        let mut entries = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            entries.push(entry.into_core(py)?);
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
#[pyclass(module = "raygeo.ops.material", name = "GridSpec")]
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

/// The folded state of one stock: an immutable snapshot.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.material", name = "MaterialState")]
pub struct PyMaterialState {
    pub(crate) inner: MaterialState,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyMaterialState {
    /// Which profile produced this state (``"prismatic"``).
    #[getter]
    fn profile(&self) -> &'static str {
        match self.inner.profile {
            FoldProfile::Prismatic => "prismatic",
            FoldProfile::Solid => "solid",
        }
    }

    /// Regions removed through the full stock thickness (world mm).
    #[getter]
    fn void_polygons(&self) -> Vec<Vec<(f64, f64)>> {
        polygons_to_tuples(&self.inner.void_polygons)
    }

    /// Removal-depth heightmap in mm, or ``None`` until depth
    /// folding lands.
    #[getter]
    fn depth_field(&self) -> Option<PyCompressedArray> {
        self.inner
            .depth_field
            .as_ref()
            .map(|d| PyCompressedArray::from_inner(d.clone()))
    }

    /// Per-pixel maximum laser power (R8), or ``None`` when no
    /// raster effects contributed.
    #[getter]
    fn surface_map(&self) -> Option<PyCompressedArray> {
        self.inner
            .surface_map
            .as_ref()
            .map(|s| PyCompressedArray::from_inner(s.clone()))
    }

    /// Grid shared by the raster outputs.
    #[getter]
    fn grid(&self) -> Option<PyGridSpec> {
        self.inner.grid.as_ref().map(|g| PyGridSpec {
            origin_mm: g.origin_mm,
            px_per_mm: g.px_per_mm,
            size_px: g.size_px,
        })
    }

    /// Sorted unique source keys whose effects were applied.
    #[getter]
    fn provenance(&self) -> Vec<String> {
        self.inner.provenance.clone()
    }

    /// First invariant violation encountered (``"top_open_violation"``
    /// or ``"solid_profile_required"``), or ``None``.
    #[getter]
    fn escalation(&self) -> Option<String> {
        self.inner.escalation.as_ref().map(|e| e.kind().to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "MaterialState(profile={:?}, voids={}, provenance={}, escalation={:?})",
            self.profile(),
            self.inner.void_polygons.len(),
            self.inner.provenance.len(),
            self.inner.escalation.as_ref().map(|e| e.kind()),
        )
    }
}

/// Fold the spec's entries against the stock into a snapshot.
///
/// Runs the prismatic fold only: through-cut classification, void
/// union clipped to the stock, the burn surface map, provenance, and
/// escalation signals. The GIL is released while folding.
#[gen_stub_pyfunction(module = "raygeo.ops.material")]
#[pyfunction(name = "fold_effects")]
fn fold_effects_py(
    py: Python<'_>,
    spec: &PyMaterialFoldSpec,
) -> PyResult<PyMaterialState> {
    let core_spec = spec.into_core(py)?;
    let state = py
        .detach(|| fold::fold_effects(&core_spec))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    Ok(PyMaterialState { inner: state })
}

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let m = PyModule::new(py, "material")?;
    m.setattr("__doc__", MODULE_DOC)?;
    m.add_class::<PyVectorEffect>()?;
    m.add_class::<PyRasterEffect>()?;
    m.add_class::<PyVolumeEffect>()?;
    m.add_class::<PyPrismaticStock>()?;
    m.add_class::<PyGridBudget>()?;
    m.add_class::<PyFoldEntry>()?;
    m.add_class::<PyMaterialFoldSpec>()?;
    m.add_class::<PyGridSpec>()?;
    m.add_class::<PyMaterialState>()?;
    m.add_function(pyo3::wrap_pyfunction!(fold_effects_py, m.clone())?)?;

    ops_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.material", &m)?;

    Ok(())
}

use std::collections::BTreeMap;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods,
};

use crate::cnc::tool::{Tool, ToolCategory, ToolMaterial, ToolModel};

pyo3_stub_gen::module_doc!("raygeo.cnc.tool", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC tool model.

Provides ToolModel (a parametric geometry parameter bag), the
ToolCategory enum (type-safe tool classification for compatibility
checks), the ToolMaterial enum, and the Tool composite. All types are
implemented in Rust and consumed by the CNC layer's signatures.
";

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let tool_mod = PyModule::new(py, "tool")?;
    tool_mod.setattr("__doc__", MODULE_DOC)?;
    tool_mod.add_class::<PyToolCategory>()?;
    tool_mod.add_class::<PyToolMaterial>()?;
    tool_mod.add_class::<PyToolModel>()?;
    tool_mod.add_class::<PyTool>()?;
    parent.add_submodule(&tool_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.tool", &tool_mod)?;

    Ok(())
}

// --- ToolCategory --------------------------------------------------------

/// Type-safe classification of a tool, for operation-compatibility
/// checks (e.g. chamfering requires ``Chamfer``/``Vbit``, slotting
/// rejects ``Probe``).
#[gen_stub_pyclass_enum]
#[pyclass(
    eq,
    module = "raygeo.cnc.tool",
    name = "ToolCategory",
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PyToolCategory {
    EndMill,
    BallNose,
    BullNose,
    Chamfer,
    Drill,
    Probe,
    Vbit,
    SlittingSaw,
    Reamer,
    Tap,
    ThreadMill,
    Dovetail,
}

impl From<ToolCategory> for PyToolCategory {
    fn from(c: ToolCategory) -> Self {
        match c {
            ToolCategory::EndMill => PyToolCategory::EndMill,
            ToolCategory::BallNose => PyToolCategory::BallNose,
            ToolCategory::BullNose => PyToolCategory::BullNose,
            ToolCategory::Chamfer => PyToolCategory::Chamfer,
            ToolCategory::Drill => PyToolCategory::Drill,
            ToolCategory::Probe => PyToolCategory::Probe,
            ToolCategory::Vbit => PyToolCategory::Vbit,
            ToolCategory::SlittingSaw => PyToolCategory::SlittingSaw,
            ToolCategory::Reamer => PyToolCategory::Reamer,
            ToolCategory::Tap => PyToolCategory::Tap,
            ToolCategory::ThreadMill => PyToolCategory::ThreadMill,
            ToolCategory::Dovetail => PyToolCategory::Dovetail,
        }
    }
}

impl From<PyToolCategory> for ToolCategory {
    fn from(c: PyToolCategory) -> Self {
        match c {
            PyToolCategory::EndMill => ToolCategory::EndMill,
            PyToolCategory::BallNose => ToolCategory::BallNose,
            PyToolCategory::BullNose => ToolCategory::BullNose,
            PyToolCategory::Chamfer => ToolCategory::Chamfer,
            PyToolCategory::Drill => ToolCategory::Drill,
            PyToolCategory::Probe => ToolCategory::Probe,
            PyToolCategory::Vbit => ToolCategory::Vbit,
            PyToolCategory::SlittingSaw => ToolCategory::SlittingSaw,
            PyToolCategory::Reamer => ToolCategory::Reamer,
            PyToolCategory::Tap => ToolCategory::Tap,
            PyToolCategory::ThreadMill => ToolCategory::ThreadMill,
            PyToolCategory::Dovetail => ToolCategory::Dovetail,
        }
    }
}

#[pymethods]
impl PyToolCategory {
    fn __repr__(&self) -> String {
        format!("ToolCategory.{}", category_name(self))
    }
}

fn category_name(c: &PyToolCategory) -> &'static str {
    match c {
        PyToolCategory::EndMill => "EndMill",
        PyToolCategory::BallNose => "BallNose",
        PyToolCategory::BullNose => "BullNose",
        PyToolCategory::Chamfer => "Chamfer",
        PyToolCategory::Drill => "Drill",
        PyToolCategory::Probe => "Probe",
        PyToolCategory::Vbit => "Vbit",
        PyToolCategory::SlittingSaw => "SlittingSaw",
        PyToolCategory::Reamer => "Reamer",
        PyToolCategory::Tap => "Tap",
        PyToolCategory::ThreadMill => "ThreadMill",
        PyToolCategory::Dovetail => "Dovetail",
    }
}

// --- ToolMaterial --------------------------------------------------------

/// Tool substrate material.
#[gen_stub_pyclass_enum]
#[pyclass(
    eq,
    module = "raygeo.cnc.tool",
    name = "ToolMaterial",
    from_py_object
)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PyToolMaterial {
    Carbide,
    HSS,
    HSSE,
    Diamond,
    CBN,
    Ceramic,
}

impl From<ToolMaterial> for PyToolMaterial {
    fn from(m: ToolMaterial) -> Self {
        match m {
            ToolMaterial::Carbide => PyToolMaterial::Carbide,
            ToolMaterial::HSS => PyToolMaterial::HSS,
            ToolMaterial::HSSE => PyToolMaterial::HSSE,
            ToolMaterial::Diamond => PyToolMaterial::Diamond,
            ToolMaterial::CBN => PyToolMaterial::CBN,
            ToolMaterial::Ceramic => PyToolMaterial::Ceramic,
        }
    }
}

impl From<PyToolMaterial> for ToolMaterial {
    fn from(m: PyToolMaterial) -> Self {
        match m {
            PyToolMaterial::Carbide => ToolMaterial::Carbide,
            PyToolMaterial::HSS => ToolMaterial::HSS,
            PyToolMaterial::HSSE => ToolMaterial::HSSE,
            PyToolMaterial::Diamond => ToolMaterial::Diamond,
            PyToolMaterial::CBN => ToolMaterial::CBN,
            PyToolMaterial::Ceramic => ToolMaterial::Ceramic,
        }
    }
}

#[pymethods]
impl PyToolMaterial {
    fn __repr__(&self) -> String {
        let name = match self {
            PyToolMaterial::Carbide => "Carbide",
            PyToolMaterial::HSS => "HSS",
            PyToolMaterial::HSSE => "HSSE",
            PyToolMaterial::Diamond => "Diamond",
            PyToolMaterial::CBN => "CBN",
            PyToolMaterial::Ceramic => "Ceramic",
        };
        format!("ToolMaterial.{name}")
    }
}

// --- ToolModel -----------------------------------------------------------

/// Parametric model describing a tool's geometry.
///
/// A single, hierarchy-free class: a bag of named parameters. Construct
/// with keyword arguments for each parameter:
///
/// .. code-block:: python
///
///    model = ToolModel(
///        diameter=6.0,
///        shank_diameter=6.0,
///        cutting_edge_height=15.0,
///        flute_count=3.0,
///        overall_length=50.0,
///    )
///
/// The type-safe tool *classification* (end-mill vs. probe vs. ...) lives
/// on :class:`Tool` as the :class:`ToolCategory` enum; a ``ToolModel``
/// only carries measurements. New geometries are created by constructing
/// a ``ToolModel`` with new parameters -- no raygeo change required.
#[gen_stub_pyclass(module = "raygeo.cnc.tool")]
#[pyclass(
    frozen,
    eq,
    module = "raygeo.cnc.tool",
    name = "ToolModel",
    skip_from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyToolModel {
    pub inner: ToolModel,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyToolModel {
    #[new]
    #[pyo3(signature = (**kwargs))]
    fn new(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut params = BTreeMap::new();
        if let Some(kw) = kwargs {
            for (k, v) in kw.iter() {
                params.insert(k.extract::<String>()?, v.extract::<f64>()?);
            }
        }
        Ok(Self {
            inner: ToolModel::new(params),
        })
    }

    fn __repr__(&self) -> String {
        let params = self
            .inner
            .params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("ToolModel({params})")
    }

    /// Read a named parameter, or ``None`` if absent.
    fn get_parameter(&self, name: &str) -> Option<f64> {
        self.inner.get(name)
    }

    /// All parameters as a ``{name: value}`` dict.
    fn get_parameters(&self) -> BTreeMap<String, f64> {
        self.inner.params.clone()
    }

    /// Cutting diameter (mm); ``0.0`` if unspecified.
    fn diameter(&self) -> f64 {
        self.inner.diameter()
    }

    /// Corner radius (mm); ``0.0`` if unspecified.
    fn corner_radius(&self) -> f64 {
        self.inner.corner_radius()
    }

    /// Cutting-edge height (mm); ``0.0`` if unspecified.
    fn cutting_edge_height(&self) -> f64 {
        self.inner.cutting_edge_height()
    }
}

// --- Tool ----------------------------------------------------------------

/// A physical cutting tool.
///
/// Combines a parametric :class:`ToolModel` (measurements), a
/// :class:`ToolCategory` (type-safe classification), a
/// :class:`ToolMaterial`, and setup parameters:
///
/// .. code-block:: python
///
///    tool = Tool(
///        label="6mm EM",
///        category=ToolCategory.EndMill,
///        model=ToolModel(diameter=6.0, ...),
///        material=ToolMaterial.Carbide,
///        stickout=15.0,
///    )
#[gen_stub_pyclass(module = "raygeo.cnc.tool")]
#[pyclass(
    frozen,
    module = "raygeo.cnc.tool",
    name = "Tool",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTool {
    pub inner: Tool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTool {
    #[new]
    #[pyo3(signature = (label, category, model, material, stickout, coating=None))]
    fn new(
        label: String,
        category: &Bound<'_, PyToolCategory>,
        model: &Bound<'_, PyToolModel>,
        material: &Bound<'_, PyToolMaterial>,
        stickout: f64,
        coating: Option<String>,
    ) -> Self {
        PyTool {
            inner: Tool {
                label,
                category: (*category.borrow()).into(),
                model: model.borrow().inner.clone(),
                material: (*material.borrow()).into(),
                stickout,
                coating,
            },
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Tool(label={:?}, category={}, diameter={})",
            self.inner.label,
            category_name(&self.inner.category.into()),
            self.diameter()
        )
    }

    /// Cutting diameter (mm).
    fn diameter(&self) -> f64 {
        self.inner.diameter()
    }

    /// Default stickout = cutting edge height + 3 mm safety.
    fn default_stickout(&self) -> f64 {
        self.inner.default_stickout()
    }

    #[getter]
    fn label(&self) -> &str {
        &self.inner.label
    }

    #[getter]
    fn category(&self) -> PyToolCategory {
        self.inner.category.into()
    }

    #[getter]
    fn material(&self) -> PyToolMaterial {
        self.inner.material.into()
    }

    #[getter]
    fn stickout(&self) -> f64 {
        self.inner.stickout
    }

    #[getter]
    fn coating(&self) -> Option<&str> {
        self.inner.coating.as_deref()
    }

    /// The tool's parametric geometry model.
    #[getter]
    fn model(&self) -> PyToolModel {
        PyToolModel {
            inner: self.inner.model.clone(),
        }
    }
}

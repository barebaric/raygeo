use std::collections::BTreeMap;

/// Classification of a tool, used for operation-compatibility checks.
///
/// For example, a chamfering operation may reject anything that is not a
/// `Chamfer` or `Vbit`, and a slotting operation may reject a `Probe`.
///
/// This is a curated taxonomy: adding a new category is a raygeo change.
/// User-defined tool geometries are expressed as new [`ToolModel`]
/// instances (a parameter bag), not new categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolCategory {
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

/// Tool substrate material.
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolMaterial {
    Carbide,
    HSS,
    HSSE,
    Diamond,
    CBN,
    Ceramic,
}

/// Parametric model describing a tool's geometry.
///
/// `ToolModel` is a single, hierarchy-free struct: a bag of named
/// parameters (e.g. `diameter`, `flute_count`, `cutting_edge_height`).
/// It is the extensible side of the tool model -- users create new
/// instances with whatever parameters their geometry requires, without
/// changing raygeo.
///
/// The type-safe classification of a tool (end-mill vs. probe vs. ...)
/// lives on [`Tool`] as the [`ToolCategory`] enum; a `ToolModel` only
/// carries measurements.
///
/// Future work: attach a parametric 3D generator (Rust trait object or a
/// Python callback) so a model can produce solid geometry from its
/// parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolModel {
    /// Named parameter values, e.g. `"diameter" -> 6.0`.
    pub params: BTreeMap<String, f64>,
}

impl ToolModel {
    /// Construct a model from a parameter map.
    pub fn new(params: BTreeMap<String, f64>) -> Self {
        Self { params }
    }

    /// Read a named parameter, if present.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.params.get(name).copied()
    }

    /// Convenience: cutting diameter (mm). `0.0` if unspecified.
    pub fn diameter(&self) -> f64 {
        self.get("diameter").unwrap_or(0.0)
    }

    /// Convenience: corner radius (mm). `0.0` if unspecified.
    pub fn corner_radius(&self) -> f64 {
        self.get("corner_radius").unwrap_or(0.0)
    }

    /// Convenience: cutting-edge height (mm). `0.0` if unspecified.
    pub fn cutting_edge_height(&self) -> f64 {
        self.get("cutting_edge_height").unwrap_or(0.0)
    }
}

/// A physical cutting tool.
///
/// Combines a parametric [`ToolModel`] (the measurements), a
/// [`ToolCategory`] (type-safe classification for compatibility checks),
/// a [`ToolMaterial`], and setup parameters.
#[derive(Clone, Debug)]
pub struct Tool {
    /// Human-readable label (e.g. "6mm EM").
    pub label: String,
    /// Type-safe classification used for operation compatibility.
    pub category: ToolCategory,
    /// Parametric geometry model (the measurements).
    pub model: ToolModel,
    /// Tool substrate material.
    pub material: ToolMaterial,
    /// Exposed stickout length (mm) past the collet.
    pub stickout: f64,
    /// Optional coating description (e.g. "TiAlN").
    pub coating: Option<String>,
}

impl Tool {
    /// Convenience: `self.model.diameter()`.
    pub fn diameter(&self) -> f64 {
        self.model.diameter()
    }

    /// Default stickout = cutting edge height + 3 mm safety.
    pub fn default_stickout(&self) -> f64 {
        self.model.cutting_edge_height() + 3.0
    }
}

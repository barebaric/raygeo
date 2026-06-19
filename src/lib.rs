//! # RayForge Geometry Library
//!
//! A 2D/3D geometry library for CAD/CAM applications. Provides structures and functions
//! for creating, manipulating, and analyzing geometric shapes including lines, arcs,
//! Bezier curves, polygons, and complex paths.
//!
//! ## Core Concepts
//!
//! - **Geometry**: A path-based geometric structure supporting Move, Line, Arc, and Bezier commands
//! - **Primitives**: Basic geometric operations like point-in-polygon, line intersections
//! - **Analysis**: Path analysis including area calculation, winding order, and tangents
//! - **Query**: Path queries for bounding boxes, distances, and closest points
//!
//! ## Usage
//!
//! ```rust
//! use raygeo::geo::Geometry;
//! use raygeo::types::Point;
//!
//! let mut geo = Geometry::new();
//! geo.move_to(0.0, 0.0, 0.0);
//! geo.line_to(10.0, 0.0, 0.0);
//! geo.line_to(10.0, 10.0, 0.0);
//! geo.close_path();
//!
//! let area = geo.area();
//! let rect = geo.rect();
//! ```

pub mod constants;
pub mod error;
pub mod geo;
pub mod image;
pub mod mesh;
pub mod ops;
pub mod prof;
pub mod svg;
pub mod types;

pub use constants::{
    CLIPPER_SCALE, EPSILON_COLLINEAR, EPSILON_GAP_CLOSE, EPSILON_INTERSECT,
    EPSILON_MEDIUM, EPSILON_MERGE, EPSILON_NEST,
};
pub use error::{AxisRepr, RaygeoError, RaygeoResult};
pub use ops::axis::Axis;
pub use ops::container::Ops;
pub use ops::enums::{CommandCategory, CommandType, SectionType};
pub use ops::group::{
    group_by_state_continuity, iter_section_ranges, iter_sections,
    segment_indices, split_into_subpaths, without_state, OpsSection,
    OpsSectionRange,
};
pub use ops::lead_in_out::apply_lead_in_out;
pub use ops::merge_lines::merge_overlapping_lines;
pub use ops::overscan::apply_overscan;
pub use ops::state::State;
pub use ops::tabs::{apply_tab_gaps, apply_tab_power, ClipPoint};
pub use ops::types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
pub use types::{
    BezierControls, BezierSplit, Command, ContourData, CubicBezier, Edge,
    GeometryPair, Point, Point3D, Polygon, Polygon3D, Rect3D, Segment3D,
    WindingOrder,
};

// ── Python bindings (behind "python" feature) ─────────────────────

/// Register one or more PyO3 functions into a module.
///
/// Eliminates the repetitive `m.add_function(wrap_pyfunction!(func, m.clone())?)?;`
/// boilerplate in every `register()` function.
#[cfg(feature = "python")]
#[macro_export]
macro_rules! register_functions {
    ($m:ident, $($func:ident),* $(,)?) => {
        $(
            $m.add_function(pyo3::wrap_pyfunction!($func, $m.clone())?)?;
        )*
    };
}

#[cfg(feature = "python")]
mod python;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3_stub_gen::define_stub_info_gatherer;

#[cfg(feature = "python")]
define_stub_info_gatherer!(stub_info);

#[cfg(feature = "python")]
pyo3_stub_gen::reexport_module_members!("raygeo" from "raygeo.geo"; "Geometry");

#[cfg(feature = "python")]
pyo3_stub_gen::reexport_module_members!("raygeo" from "raygeo.ops"; "Ops");

#[cfg(feature = "python")]
pyo3_stub_gen::module_doc!("raygeo", "{}", MODULE_DOC);

/// Module documentation string used for Python `__doc__`.
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) const MODULE_DOC: &str = concat!(
    "RayGeo — 2D/3D geometry engine for CAD/CAM applications.\n",
    "\n",
    "Core features:\n",
    "- Geometry types: points, lines, arcs, circles, beziers, polygons, rectangles\n",
    "- Path analysis: length, area, bounding box, containment, intersection\n",
    "- Path manipulation: offset, clipping, fitting, simplification, smoothing\n",
    "- Minkowski sums for toolpath generation\n",
    "- Command sequence (Ops) for CNC motion control\n",
    "- Serialization to/from industry formats\n",
    "\n",
    "Submodules:\n",
    "- raygeo.geo — Geometry and path/shape/algo operations\n",
    "- raygeo.ops — Command sequence (Ops) manipulation\n",
    "\n",
    "Examples:\n",
    "    Creating and inspecting geometry:\n",
    "\n",
    "    >>> from raygeo.geo import Geometry\n",
    "    >>> geom = Geometry()\n",
    "    >>> geom.add_rect(0, 0, 100, 50)\n",
    "    >>> geom.add_circle(50, 25, 10)\n",
    "    >>> geom.area()\n",
    "    5000.0 - 314.159...\n",
    "    >>> len(geom)\n",
    "    2\n",
    "\n",
    "    Manipulating command sequences:\n",
    "\n",
    "    >>> from raygeo.ops import Ops, Command\n",
    "    >>> ops = Ops()\n",
    "    >>> ops.set_power(1.0)\n",
    "    >>> ops.move_to(0, 0)\n",
    "    >>> ops.line_to(100, 0)\n",
    "    >>> ops.travel_distance()\n",
    "    100.0",
);

/// Python extension module entry-point.
#[cfg(feature = "python")]
#[pymodule(gil_used = false)]
fn raygeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::geo::register(m)?;
    python::image::register(m)?;
    python::mesh::register(m)?;
    python::ops::register(m)?;
    python::svg::register(m)?;
    python::svg::register(m)?;

    Ok(())
}

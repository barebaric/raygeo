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
//! use raygeo::{Geometry, Point};
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
pub mod nest;
pub mod ops;
pub mod svg;
pub mod types;

pub use constants::*;
pub use error::*;
pub use geo::*;
pub use ops::axis::Axis;
pub use ops::container::*;
pub use ops::enums::*;
pub use ops::group::*;
pub use ops::state::*;
pub use ops::types::*;
pub use types::*;

/// Register one or more PyO3 functions into a module.
///
/// Eliminates the repetitive `m.add_function(wrap_pyfunction!(func, m.clone())?)?;`
/// boilerplate in every `register()` function.
#[macro_export]
macro_rules! register_functions {
    ($m:ident, $($func:ident),* $(,)?) => {
        $(
            $m.add_function(pyo3::wrap_pyfunction!($func, $m.clone())?)?;
        )*
    };
}

mod python;

use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

define_stub_info_gatherer!(stub_info);

pyo3_stub_gen::reexport_module_members!("raygeo" from "raygeo.geo"; "Geometry");
pyo3_stub_gen::reexport_module_members!("raygeo" from "raygeo.ops"; "Ops");

pyo3_stub_gen::module_doc!("raygeo", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = concat!(
    "RayGeo — 2D/3D geometry engine for laser cutting and CAM applications.\n",
    "\n",
    "Core features:\n",
    "- Geometry types: points, lines, arcs, circles, beziers, polygons, rectangles\n",
    "- Path analysis: length, area, bounding box, containment, intersection\n",
    "- Path manipulation: offset, clipping, fitting, simplification, smoothing\n",
    "- Minkowski sums for toolpath generation\n",
    "- Command sequence (Ops) for laser cutter motion control\n",
    "- Serialization to/from industry formats\n",
    "\n",
    "Submodules:\n",
    "- raygeo.geo — Geometry and path/shape/algo operations\n",
    "- raygeo.ops — Command sequence (Ops) manipulation\n",
    "\n",
    "Examples:\n",
    "    Creating and inspecting geometry:\n",
    "\n",
    "    >>> from raygeo import Geometry\n",
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
    "    >>> ops.set_speed(100)\n",
    "    >>> ops.move_to(0, 0)\n",
    "    >>> ops.line_to(100, 0)\n",
    "    >>> ops.travel_distance()\n",
    "    100.0",
);

#[pymodule(gil_used = false)]
fn raygeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::geo::register(m)?;
    python::image::register(m)?;
    python::nest::register(m)?;
    python::ops::register(m)?;
    python::svg::register(m)?;
    // Backward-compat re-exports on root
    m.add("Geometry", m.getattr("geo")?.getattr("Geometry")?)?;
    m.add("Ops", m.getattr("ops")?.getattr("Ops")?)?;
    m.add("Rect", m.getattr("geo")?.getattr("types")?.getattr("Rect")?)?;
    Ok(())
}

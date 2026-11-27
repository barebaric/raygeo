//! # RayGeo Geometry Library
//!
//! A 2D/3D geometry library for CAD/CAM applications. Provides
//! structures and functions for creating, manipulating, and analyzing
//! geometric shapes including lines, arcs, Bezier curves, polygons, and
//! complex paths.
//!
//! ## Layered architecture
//!
//! The crate is split into layers that depend only downward:
//!
//! ```text
//!   geo   →   ops   →   cnc
//!     └→   sketch
//!                        │
//!                        ↓ depends on
//!                    pipeline (generic runtime; std + rayon only)
//! ```
//!
//! **[`geo`]** — Pure geometry.
//! Primitives & geometric algorithms: points, paths, offsets, medial
//! axes, clearing-state tracking, adaptive entry/wavefront generation.
//! Knows nothing about machining, motion commands, tools, or feed
//! rates.
//!
//! **[`sketch`]** — Sketch constraint solving.
//! Moves a set of 2D points so that geometric constraints (horizontal,
//! distance, tangency, ...) hold, via trust-region-reflective least
//! squares. Knows only [`geo`] primitives; no machining concepts.
//!
//! **[`ops`]** — Motion assembly.
//! Turns geometric primitives into [`Ops`] command sequences: linking,
//! classification (cut vs travel), lead-in/out, overscan, raster fill,
//! peeling strategy.  Holds the generic [`State`] representation
//! (feed_rate, rapid_rate, …) but does NOT decide what values to use —
//! those are passed in by the caller.  Also hosts the format converters
//! under [`ops::convert`], including the G-code, Epilog and Ruida
//! machine-code encoders.
//!
//! **[`cnc`]** — Orchestration.
//! Follows a strict three-stage pipeline:
//! 1. **Plan** (`cnc::plan`): descriptive `Plan` (a sequence of
//!    `PlanStep`s, each carrying an `Assembler` spec).  Planners like
//!    `plan_clearing` and `plan_entry` produce Plans.
//! 2. **Intent** (`cnc::execution::intent`): `create_intent(plan, part)`
//!    converts a Plan into executable `NodeRequest`s with state
//!    threading and a final aggregate.
//! 3. **Execute** (`pipeline::execute`): `execute_stages` runs the
//!    intent tree on a rayon thread pool; the final `Aggregate` node
//!    produces the linked [`Ops`].
//!
//! **[`pipeline`]** — Generic runtime executor.
//! Executes an intent tree of `NodeRequest`s on a rayon thread pool.
//! Knows nothing about CNC, ops, or geometry — it runs generic
//! [`Compute`] and [`Aggregate`] trait objects and passes opaque
//! `Box<dyn Any>` outputs between nodes. Depends only on `std` and
//! `rayon`.
//!
//! ### Key constraint
//!
//! Ops-layer assemblers **must produce and consume [`Ops`]** — never
//! raw polygon/polyline lists or Z-encoded point arrays.  Motion
//! classification is encoded as `MoveTo` (rapid/travel) vs `LineTo`
//! (feed/cut) at the command level.  `State` values are passed in by
//! the caller; assemblers apply them via [`Ops::apply_state`] but never
//! compute them.
//!
//! When adding new functionality, ask: *does this decide what to cut,
//! in what order, or how fast?*  If yes, it belongs in `ops`.  If it
//! only computes shapes, distances, or geometric relationships, it
//! belongs in `geo`.
//!
//! ## Core Concepts
//!
//! - **Geometry**: path-based structure with Move/Line/Arc/Bezier
//! - **Primitives**: point-in-polygon, line intersections
//! - **Analysis**: area, winding order, tangents
//! - **Query**: bounding boxes, distances, closest points
//!
//! ## Usage
//!
//! ```rust
//! use raygeo::geo::Geometry;
//! use raygeo::geo::types::Point;
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

pub mod barcode;
pub mod cnc;
pub mod compressed_array;
pub mod constants;
pub mod driver;
pub mod error;
pub mod formats;
pub mod fstring;
pub mod geo;
pub mod image;
pub mod log;
pub mod mesh;
pub mod ops;
pub mod pipeline;
pub mod prof;
pub mod sketch;
pub mod template;
// The tracing machinery is only driven from the Python bindings; the
// pure Rust core compiles it but has no caller.
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) mod trace;
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) mod trace_types;
pub mod utils;

// The M2 WebSocket client needs a rustls trust store, selected at
// build time. Both provider features imply `m2`; this guard catches
// `--no-default-features --features m2` with an actionable message
// instead of a cryptic missing-`Connector` variant error.
#[cfg(all(
    feature = "m2",
    not(any(feature = "tls-native-roots", feature = "tls-webpki-roots"))
))]
compile_error!(
    "the `m2` feature requires a TLS root provider: enable \
     `tls-native-roots` (OS trust store, the default) or \
     `tls-webpki-roots` (bundled Mozilla roots)"
);

pub use constants::{
    CLIPPER_SCALE, EPSILON_BOUNDARY, EPSILON_COLLINEAR, EPSILON_GAP_CLOSE,
    EPSILON_INTERSECT, EPSILON_MEDIUM, EPSILON_MERGE, EPSILON_NEST,
};
pub use error::{AxisRepr, RaygeoError, RaygeoResult};

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
pyo3_stub_gen::module_doc!("raygeo", "{}", MODULE_DOC);

/// Module documentation string used for Python `__doc__`.
#[cfg(feature = "python")]
pub(crate) const MODULE_DOC: &str = concat!(
    "RayGeo — 2D/3D geometry, CAM and machine-driver engine.\n",
    "\n",
    "Layered architecture\n",
    "--------------------\n",
    "\n",
    "The crate is split into layers that depend only downward::\n",
    "\n",
    "    geo  →  ops  →  cnc\n",
    "      └→  sketch\n",
    "    geo  →  formats\n",
    "    pipeline  (generic runtime; cnc depends on it)\n",
    "    driver    (transports and protocol sessions)\n",
    "\n",
    "``geo`` — Pure geometry.\n",
    "    Primitives & geometric algorithms: points, paths, offsets,\n",
    "    medial axes, clearing-state tracking, adaptive entry/wavefront\n",
    "    generation.  Knows nothing about machining, motion commands,\n",
    "    tools, or feed rates.\n",
    "\n",
    "``sketch`` — Sketch constraint solving.\n",
    "    Moves 2D points until geometric constraints (horizontal,\n",
    "    distance, tangency, ...) hold, using trust-region-reflective\n",
    "    least squares.  Depends only on ``geo``; no machining concepts.\n",
    "\n",
    "``ops`` — Motion assembly.\n",
    "    Turns geometric primitives into ``Ops`` command sequences.\n",
    "    Linking, classification (cut vs travel), lead-in/out, overscan,\n",
    "    raster fill, peeling strategy.  Holds the generic ``State``\n",
    "    representation (feed_rate, rapid_rate, …) but does NOT decide\n",
    "    what values to use — those are passed in by the caller.  Also\n",
    "    hosts the format converters under ``ops.convert``, including\n",
    "    the G-code, Epilog and Ruida machine-code encoders.\n",
    "\n",
    "``cnc`` — Orchestration (Plan → Intent → Execute).\n",
    "    Plan: descriptive Plans (sequences of PlanSteps, each an\n",
    "    Assembler spec).  Intent: create_intent() converts a Plan into\n",
    "    executable NodeRequests.  Execute: run_intent() runs the\n",
    "    pipeline and returns the final linked Ops.\n",
    "\n",
    "``pipeline`` — Generic runtime.\n",
    "    Runs an intent tree of nodes on a thread pool. Knows nothing\n",
    "    about CNC or ops — purely generic Compute/Aggregate dispatch.\n",
    "\n",
    "``formats`` — File-format import and export.\n",
    "    CorelDRAW, SVG, G-code and DXF readers/writers; depends only on\n",
    "    ``geo``.\n",
    "\n",
    "``driver`` — Device transports and protocol sessions.\n",
    "    GRBL, Marlin, Smoothie, ESP3D, OctoPrint, xTool, Epilog, Ruida,\n",
    "    the simulator and the remote daemon, exposed as\n",
    "    ``raygeo.driver``.  Pure Rust transports and sessions;\n",
    "    machine-code encoders live in ``ops.convert``, not here.\n",
    "\n",
    "Key constraint: ops-layer assemblers always produce/consume ``Ops``\n",
    "objects, never raw polygon or polyline lists.  Motion classification\n",
    "is encoded as ``MoveTo`` (travel) vs ``LineTo`` (cut) at the command\n",
    "level.\n",
    "\n",
    "Core features\n",
    "-------------\n",
    "- Geometry types: points, lines, arcs, circles, beziers, polygons\n",
    "- Path analysis: length, area, bounding box, containment, intersection\n",
    "- Path manipulation: offset, clipping, fitting, simplification, smoothing\n",
    "- Minkowski sums for toolpath generation\n",
    "- Command sequence (Ops) for CNC motion control\n",
    "- Sketch constraint solving and topology\n",
    "- CNC plans, intents and rayon-threadpool execution\n",
    "- Machine-code conversion (G-code, Epilog, Ruida)\n",
    "- Serialization to/from industry formats\n",
    "- Native machine drivers and a remote daemon\n",
    "- Generic intent-tree pipeline (rayon-threadpool execution)\n",
    "\n",
    "Submodules\n",
    "----------\n",
    "- raygeo.geo — Geometry and path/shape/algo operations\n",
    "- raygeo.ops — Command sequence (Ops) manipulation and motion assembly\n",
    "- raygeo.ops.convert — Format converters and machine-code encoders\n",
    "- raygeo.sketch — Constraint solving, entities and topology\n",
    "- raygeo.formats — File-format import and export (CorelDRAW, SVG, G-code, DXF)\n",
    "- raygeo.cnc — CNC orchestration (Plans, Intents, pipeline glue)\n",
    "- raygeo.pipeline — Generic runtime intent-tree executor\n",
    "- raygeo.driver — Machine drivers, discovery and remote daemon\n",
    "- raygeo.template — Job counter and template placeholder scanning\n",
    "\n",
    "Command line\n",
    "------------\n",
    "The package installs ``raygeo`` for adaptive-clearing tools and\n",
    "``raydriver`` for machine and daemon tools.\n",
    "\n",
    "Examples\n",
    "--------\n",
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
    "    >>> from raygeo.ops import Ops\n",
    "    >>> ops = Ops()\n",
    "    >>> ops.set_power(1.0)\n",
    "    >>> ops.move_to(0, 0, 0)\n",
    "    >>> ops.line_to(100, 0, 0)\n",
    "    >>> ops.distance()\n",
    "    100.0",
);

/// Python extension module entry-point.
#[cfg(feature = "python")]
#[pymodule(gil_used = false)]
fn raygeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::barcode::register(m)?;
    python::compressed_array::register(m)?;
    python::cnc::register(m)?;
    python::fstring::register(m)?;
    python::formats::register(m)?;
    python::geo::register(m)?;
    python::image::register(m)?;
    python::mesh::register(m)?;
    python::ops::register(m)?;
    python::sketch::register(m)?;
    python::template::register(m)?;
    python::trace::register(m)?;
    python::pipeline::register(m)?;
    python::driver::register(m)?;
    python::driver::register_root(m)?;

    Ok(())
}

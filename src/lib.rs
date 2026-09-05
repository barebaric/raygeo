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
//! **[`ops`]** — Motion assembly.
//! Turns geometric primitives into [`Ops`] command sequences: linking,
//! classification (cut vs travel), lead-in/out, overscan, raster fill,
//! peeling strategy.  Holds the generic [`State`] representation
//! (feed_rate, rapid_rate, …) but does NOT decide what values to use —
//! those are passed in by the caller.
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

/// Global allocator.
///
/// mimalloc returns freed pages to the OS eagerly and fragments far
/// less than glibc malloc, which keeps the process RSS near the live
/// data size instead of the historical allocation peak.
///
/// Purges run immediately: mimalloc's default 10 ms purge delay keeps
/// the churn pages from a pipeline run committed long enough to stick
/// in RSS (measured ~500 MB higher in the rayforge app).
///
/// Disabled on FreeBSD: the mimalloc static TLS usage exceeds the
/// TLS surplus available to shared Python extension modules, making
/// `import raygeo` fail with "No space available for static Thread
/// Local Storage".  FreeBSD uses the system allocator instead
/// (see rayforge issue #389).
#[cfg(not(target_os = "freebsd"))]
mod allocator {
    use std::alloc::{GlobalAlloc, Layout};
    use std::sync::Once;

    /// `mi_option_purge_delay` in the mimalloc v3 option enum (not
    /// exposed by libmimalloc-sys): delay in ms before freed memory is
    /// purged back to the OS; 0 = immediate.
    const MI_OPTION_PURGE_DELAY: libmimalloc_sys::mi_option_t = 15;

    static CONFIGURE_MIMALLOC: Once = Once::new();

    fn configure_mimalloc() {
        CONFIGURE_MIMALLOC.call_once(|| unsafe {
            libmimalloc_sys::mi_option_set(MI_OPTION_PURGE_DELAY, 0);
        });
    }

    struct RaygeoAlloc;

    unsafe impl GlobalAlloc for RaygeoAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            configure_mimalloc();
            mimalloc::MiMalloc.alloc(layout)
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            mimalloc::MiMalloc.dealloc(ptr, layout)
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            configure_mimalloc();
            mimalloc::MiMalloc.alloc_zeroed(layout)
        }

        unsafe fn realloc(
            &self,
            ptr: *mut u8,
            layout: Layout,
            new_size: usize,
        ) -> *mut u8 {
            configure_mimalloc();
            mimalloc::MiMalloc.realloc(ptr, layout, new_size)
        }
    }

    #[global_allocator]
    static GLOBAL: RaygeoAlloc = RaygeoAlloc;
}

pub mod cnc;
pub mod compressed_array;
pub mod constants;
pub mod error;
pub mod fstring;
pub mod geo;
pub mod image;
pub(crate) mod log;
pub mod mesh;
pub mod ops;
pub mod pipeline;
pub mod prof;
pub mod svg;
pub(crate) mod trace_types;

pub(crate) mod trace;
pub mod utils;

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
    "RayGeo — 2D/3D geometry engine for CAD/CAM applications.\n",
    "\n",
    "Layered architecture\n",
    "--------------------\n",
    "\n",
    "The crate is split into layers that depend only downward::\n",
    "\n",
    "    geo  →  ops  →  cnc   →  pipeline  (runtime)\n",
    "\n",
    "``geo`` — Pure geometry.\n",
    "    Primitives & geometric algorithms: points, paths, offsets,\n",
    "    medial axes, clearing-state tracking, adaptive entry/wavefront\n",
    "    generation.  Knows nothing about machining, motion commands,\n",
    "    tools, or feed rates.\n",
    "\n",
    "``ops`` — Motion assembly.\n",
    "    Turns geometric primitives into ``Ops`` command sequences.\n",
    "    Linking, classification (cut vs travel), lead-in/out, overscan,\n",
    "    raster fill, peeling strategy.  Holds the generic ``State``\n",
    "    representation (feed_rate, rapid_rate, …) but does NOT decide\n",
    "    what values to use — those are passed in by the caller.\n",
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
    "- Serialization to/from industry formats\n",
    "- Generic intent-tree pipeline (rayon-threadpool execution)\n",
    "\n",
    "Submodules\n",
    "----------\n",
    "- raygeo.geo — Geometry and path/shape/algo operations\n",
    "- raygeo.ops — Command sequence (Ops) manipulation and motion assembly\n",
    "- raygeo.cnc — CNC orchestration (Plans, Intents, pipeline glue)\n",
    "- raygeo.pipeline — Generic runtime intent-tree executor\n",
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
    python::compressed_array::register(m)?;
    python::cnc::register(m)?;
    python::fstring::register(m)?;
    python::geo::register(m)?;
    python::image::register(m)?;
    python::mesh::register(m)?;
    python::ops::register(m)?;
    python::svg::register(m)?;
    python::trace::register(m)?;
    python::pipeline::register(m)?;

    Ok(())
}

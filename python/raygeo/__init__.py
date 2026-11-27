"""RayGeo — 2D/3D geometry, CAM and machine-driver engine.

Layered architecture
--------------------

The crate is split into layers that depend only downward::

    geo  →  ops  →  cnc
      └→  sketch
    geo  →  formats
    pipeline  (generic runtime; cnc depends on it)
    driver    (transports and protocol sessions)

``geo`` — Pure geometry.
    Primitives & geometric algorithms: points, paths, offsets,
    medial axes, clearing-state tracking, adaptive entry/wavefront
    generation.  Knows nothing about machining, motion commands,
    tools, or feed rates.

``sketch`` — Sketch constraint solving.
    Moves 2D points until geometric constraints hold, using
    trust-region-reflective least squares. Depends only on ``geo``.

``ops`` — Motion assembly.
    Turns geometric primitives into ``Ops`` command sequences.
    Linking, classification (cut vs travel), lead-in/out, overscan,
    raster fill, peeling strategy. Holds the generic ``State``
    representation (feed_rate, rapid_rate, …) but does NOT decide
    what values to use — those are passed in by the caller. Also
    hosts the format converters under ``ops.convert``, including
    the G-code, Epilog and Ruida machine-code encoders.

``cnc`` — Orchestration (Plan → Intent → Execute).
    Plan: descriptive Plans (sequences of PlanSteps, each an
    Assembler spec). Intent: create_intent() converts a Plan into
    executable NodeRequests. Execute: run_intent() runs the
    pipeline and returns the final linked Ops.

``pipeline`` — Generic runtime.
    Runs an intent tree of nodes on a thread pool. Knows nothing
    about CNC or ops — purely generic Compute/Aggregate dispatch.

``formats`` — File-format import and export.
    CorelDRAW, SVG, G-code and DXF readers/writers; depends only on
    ``geo``.

``driver`` — Device transports and protocol sessions.
    GRBL, Marlin, Smoothie, ESP3D, OctoPrint, xTool, Epilog, Ruida,
    the simulator and the remote daemon, exposed as
    ``raygeo.driver``. Pure Rust transports and sessions;
    machine-code encoders live in ``ops.convert``, not here.

Key constraint: ops-layer assemblers always produce/consume ``Ops``
objects, never raw polygon or polyline lists. Motion classification
is encoded as ``MoveTo`` (travel) vs ``LineTo`` (cut) at the command
level.

Core features
-------------
- Geometry types: points, lines, arcs, circles, beziers, polygons
- Path analysis: length, area, bounding box, containment, intersection
- Path manipulation: offset, clipping, fitting, simplification,
  smoothing
- Minkowski sums for toolpath generation
- Command sequence (Ops) for CNC motion control
- Sketch constraint solving and topology
- CNC plans, intents and rayon-threadpool execution
- Machine-code conversion (G-code, Epilog, Ruida)
- Serialization to/from industry formats
- Native machine drivers and a remote daemon
- Generic intent-tree pipeline (rayon-threadpool execution)

Submodules
----------
- raygeo.geo — Geometry and path/shape/algo operations
- raygeo.ops — Command sequence (Ops) manipulation and motion assembly
- raygeo.ops.convert — Format converters and machine-code encoders
- raygeo.sketch — Constraint solving, entities and topology
- raygeo.formats — File-format import and export (CorelDRAW, SVG,
  G-code, DXF)
- raygeo.cnc — CNC orchestration (Plans, Intents, pipeline glue)
- raygeo.pipeline — Generic runtime intent-tree executor
- raygeo.driver — Machine drivers, discovery and remote daemon
- raygeo.template — Job counter and template placeholder scanning

Command line
------------
The package installs ``raygeo`` for adaptive-clearing tools and
``raydriver`` for machine and daemon tools.

Examples
--------
    Creating and inspecting geometry:

    >>> from raygeo.geo import Geometry
    >>> geom = Geometry()
    >>> geom.add_rect(0, 0, 100, 50)
    >>> geom.add_circle(50, 25, 10)
    >>> geom.area()
    5000.0 - 314.159...
    >>> len(geom)
    2

    Manipulating command sequences:

    >>> from raygeo.ops import Ops
    >>> ops = Ops()
    >>> ops.set_power(1.0)
    >>> ops.move_to(0, 0, 0)
    >>> ops.line_to(100, 0, 0)
    >>> ops.distance()
    100.0
"""

import raygeo.raygeo as _raygeo  # type: ignore[import-untyped]
from raygeo.enums import CutOrder, CutSide


def __getattr__(name):
    return getattr(_raygeo, name)


__all__ = [
    "CutSide",
    "CutOrder",
]

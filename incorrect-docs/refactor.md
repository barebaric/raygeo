# Refactor: cherry-pick pure-geometry helpers out of `ops/cut/`, and generalize the tracing framework

## Motivation

Per the layering rules in `cnc-module.md`:

```
geo  →  ops  →  cnc      (never import upward)
```

| Layer | Owns | Does NOT know |
| ----- | ---- | ------------- |
| `geo` | Primitives & pure geometric algorithms. No "what-to-cut" decisions, no motion verbs, no machining terminology, no `Ops`. | machining, motion, tools, `Ops` |
| `ops` | Motion assembly: clearing strategies, linking, classification, and Ops emission. All assemblers produce and consume `Ops`. | tools, spindle, feed values |
| `cnc` | Operation orchestration: sequences entry + clear + finish, resolves tool-aware `State` via `StateStrategy`. Does not exist yet. | geometry algorithms |

### What does NOT move (audit conclusions)

The `ops/cut/` directory was audited function-by-function against the
"no motion verbs" test for the `geo` layer. **Most of `ops/cut/`
correctly stays in `ops/`:**

- `ClearedArea` (`cleared_area.rs`) — the `batch_path: Vec<Point>`
  field and `expand_step(prev, next, radius)` / `expand_batched`
  methods take tool-centre *motion segments* as input. The struct
  tracks the result of motion, not just a shape.
- `stepper::step` + `StepperOptions` (`stepper.rs`) — choose the
  tool's next heading. Explicit motion decisions; the entire file
  exists to answer "which way does the tool turn next".
- `interp::Interpolation` (the bracket/root-find solver in
  `interp.rs`) — finds the steering angle that meets the target
  cut-area-per-distance. Optimisation over motion directions.
- `search::search_frontier_engagement`, `walk_polygon_samples`
  (`search.rs`) — walk the frontier to pick a *place for the tool to
  resume cutting*. `heading` is a tangent direction of travel.
- `crescent::cut_area` (`crescent.rs`) — `SweepContext` is
  motion-shaped (the c2 disc, fragments, valid-area for a tool step
  from `c1` to `c2`). The geometry is real, but the input/output
  model is "tool moves from c1 to c2 — what area does it sweep?".
- `types::ToolPose = (pos, heading)` — a tool pose, not a shape.

The only thing in `ops/cut/` that **unambiguously** qualifies as pure
geometry is `interp::point_in_valid_area(pt, area)` — a polygon
shells-vs-holes containment test with zero motion concepts. (closely
related to existing primitives in `geo::shape::polygon`.)

Conclusion: `ops/cut/` is correctly located as a directory; only
`point_in_valid_area` is in the wrong layer. The "ops must produce
`Ops`" rule is imperfectly satisfied by `ops/cut/`, but moving
motion-aware primitives into `geo/` to satisfy the rule would be a
worse violation. The rule wording may be relaxed later — *not* a code
change for this refactor.

### What does NOT move to `cnc/`

Nothing. `cnc-module.md` reserves the `cnc/` layer for *operation
orchestration* (sequences entry + clear + finish; resolves
tool-aware `State` via `StateStrategy`). The existing
`adaptive_clearing()` is a single motion-assembly strategy, not an
orchestrator: it takes a caller-supplied `State` and emits one `Ops`.
The future `cnc/` layer (Phases D/E of `cnc-module.md`) builds on top
of — not by extracting from — what's already correctly in `ops/`.

### The tracer needs generalizing

The tracing infrastructure (`src/trace.rs` + the recorder in
`src/ops/assembly/adaptive/trace.rs` + the Python reader in
`python/raygeo/cli/trace.py`) is the only piece of the existing code
that genuinely is over-coupled to a single operation. The binary
header is `"ADPT"` (adaptive-clearing), `TraceContext` always carries
pocket boundary + islands + seeds, `MatTrace` always writes an MAT
block, and the `TraceRecord` schema has 28 adaptive-clearing-specific
flat fields (`iters`, `eng_*`, `wall_hug_*`,
`resume_candidate_points`, …) hard-coded into both the Rust struct
and the Python `__slots__` reader.

Future operations (`adaptive_entry`, `adaptive_wavefront`, the HSM
orchestrator in `cnc-module.md` Phase E) need to be able to emit
trace files inspectable with `raygeo inspect`, without their own
fields leaking into the adaptive record, and without forcing every
reader to know every op's fields. This refactor generalises the
tracer while preserving the existing adaptive format on disk (no
backward-compat break, no `.bin` file format bump in this PR).

## Target architecture

```
geo (pure geometry)
  └── shape/polygon.rs  · point_in_valid_area (added)

ops (motion assembly — directory structure unchanged)
  ├── cut/                       (unchanged — point_in_valid_area moves out)
  │   ├── cleared_area.rs
  │   ├── crescent.rs
  │   ├── stepper.rs
  │   ├── search.rs
  │   ├── interp.rs               · now calls crate::geo::shape::polygon::point_in_valid_area
  │   ├── types.rs
  │   └── mod.rs
  ├── assembly/adaptive/         (unchanged structure)
  │   └── trace.rs               · rewritten to the new Tracer API
  └── assembly/polyline.rs raster/ entry.rs wavefront.rs (unchanged)

trace/ (NEW — generic binary-writer service at crate root)
  └── (moved from src/trace.rs, refactored to op-agnostic I/O)

ops/assembly/adaptive/trace.rs (the adaptive op-specific recorder)
  · owns adaptive payload, adaptive binary blocks
  · uses trace::Tracer for the binary file layout only

python/raygeo/cli/trace.py
  · op-dispatching reader; adaptive payload handler registered for OpType::AdaptiveClearing
```

A crate-root `trace` module continues to own the *how* of writing a
binary trace file. The *what* (record schema, op-specific binary
blocks) moves into per-op modules — starting with `adaptive/trace.rs`
which becomes the reference implementation of an "op trace recorder".

## Dependencies (import map after refactor)

```
geo                                              (pure geometry)
  │
  ├──→ (self)

ops                                              (motion assembly)
  ├──→ geo                                       (geometry primitives incl. new point_in_valid_area)
  └──→ trace                                      (binary-writer API; opaque record-shaped API)

trace                                            (crate-root binary-writer service)
  └──→ (std / serde / rmp-serde only — no crate::ops, no crate::geo)
```

- `ops/assembly/adaptive/trace.rs` uses `trace::Tracer` for binary
  I/O. **No** `crate::geo::*` (or only `MedialAxis` for the MAT
  block) and **no** `crate::ops::*` — the op recorder is the
  boundary between domain data and binary serialisation.
- `src/cnc/` does *not* exist after this refactor.
- `adaptive_clearing` Python surface (`raygeo.ops.assembly.adaptive.*`,
  `raygeo.ops.cut.*`) is unchanged. No Python user-facing breaking
  changes.

## Guiding principle

**After every step:** `make format && make lint && make stubs && make docs && make test` passes; trace files produced before the refactor still load; `raygeo trace` → `raygeo print` → `raygeo inspect` round-trips byte-equivalent to before. Every step is independently shippable.

---

# Phase A — Cherry-pick `point_in_valid_area` to `geo/`

The only genuine `geo`-layer candidate found by the audit.

## A1. Add `point_in_valid_area` to `geo/shape/polygon.rs`

- Add a new public function `point_in_valid_area(pt: Point, area: &[Polygon]) -> bool` in `src/geo/shape/polygon.rs` with the same body as the current `ops::cut::interp::point_in_valid_area`:
  ```rust
  pub fn point_in_valid_area(pt: Point, area: &[Polygon]) -> bool {
      let mut inside_outer = false;
      let mut inside_hole = false;
      for poly in area {
          if poly.len() < 3 { continue; }
          let is_ccw = get_polygon_signed_area(poly) > 0.0;
          let inside = is_point_in_polygon(pt, poly);
          if is_ccw && inside { inside_outer = true; }
          else if !is_ccw && inside { inside_hole = true; }
      }
      inside_outer && !inside_hole
  }
  ```
  Both helpers (`get_polygon_signed_area`, `is_point_in_polygon`) are already in `geo/shape/polygon.rs`, so the new function is self-contained.
- Add a `#[prof]` attribute to match the original.

## A2. Replace `ops/cut/interp.rs` definition with a re-export

- Remove the body of `point_in_valid_area` from `src/ops/cut/interp.rs`.
- Replace with `pub use crate::geo::shape::polygon::point_in_valid_area;` (keeps the existing call sites `crate::ops::cut::interp::point_in_valid_area` working without touching them — minimal-diff choice; the re-export can be removed in a later cleanup PR if desired).
- Alternatively, if the project prefers to update call sites directly, rewrite the 4 callers (`resume.rs`, `resume_segment.rs`, `resume_wall_hug.rs`, `resume_island.rs`) to import `crate::geo::shape::polygon::point_in_valid_area` and delete the `ops/cut/interp::point_in_valid_area` re-export. Pick the approach that matches the codebase's existing convention for "moved helper" — `git log` on a recent move will show.

## A3. Python binding

The existing Python binding `src/python/ops/cut/interp.rs` exposes `point_in_valid_area` under `raygeo.ops.cut.interp`. **Keep it.** Don't add a new `raygeo.geo.shape.polygon.point_in_valid_area` Python binding unless the `tools/examples/ops_cut_interp.py` example references it — and the audit shows it imports from `raygeo.ops.cut.interp`, not from `raygeo.geo.*`. The Python surface is unchanged.

## A4. Verify

```
make format && make lint && make stubs && make docs && make test
```

Then confirm:

- `rg point_in_valid_area src/` — definition now in `geo/shape/polygon.rs`, re-export (or call-site updates) elsewhere.
- `raygeo.ops.cut.interp.point_in_valid_area` still importable from Python (test file `tests/geo/clearing/...` — wait, the tests live under `tests/ops/cut/test_ops_cut_interp.py`; that's unchanged).
- All `tests/ops/cut/test_ops_cut_interp.py` tests still pass.

---

# Phase B — Refactor the binary tracer to be op-agnostic

Goal: keep the on-disk format and Python experience for adaptive
clearing **byte-identical**. Future ops get a clean way to plug in
their own block layout and record payload.

## Scope

The current `src/trace.rs` mixes three responsibilities:

1. **Generic** binary-writer plumbing: open/create, 12-byte header
   (magic + reserved + record-count), length-prefixed msgpack record
   buffer, Drop-implements-flush-on-panic protocol, finish-and-patch
   record-count.
2. **Adaptive-clearing-specific** binary blocks: `TraceContext`
   (tool_radius, boundary, islands, seeds), `MatTrace` (MAT nodes
   / edges / clearance / root), and the `toolpath` block of
   `TracePoint {x, y, is_travel}`.
3. The per-step `TraceRecord` schema lives in
   `ops/assembly/adaptive/trace.rs` but mixes ~12 generic fields with
   ~16 adaptive-specific ones.

After the refactor:

- `crate::trace` owns (1) only — a small, op-agnostic binary
  container API.
- `ops/assembly/adaptive/trace.rs` owns (2) and (3) — the adaptive
  block writers and the adaptive record payload get serialised into
  the generic container by the op-specific recorder.
- `python/raygeo/cli/trace.py` stays the format-specific reader it is
  (it still knows it's reading an adaptive-clearing file). The Phase
  C generalisation below is a possible follow-up — *not done here*.

## B1. Slim `src/trace.rs` down to a generic container

Keep only what is truly op-agnostic:

```rust
// src/trace.rs (after Phase B1)

use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use serde::Serialize;

pub(crate) const TRACE_MAGIC: [u8; 4] = *b"ADPT";

pub(crate) struct Tracer {
    file: std::fs::File,
    count: u32,
    records: Vec<u8>,
    finalized: bool,
}

impl Tracer {
    /// Open `path`, write the 12-byte header, then write the caller-
    /// supplied `preamble` bytes verbatim.  The preamble is whatever
    /// op-specific binary blocks (geometry, MAT, toolpath, …) the
    /// op recorder wants to embed before the record stream.
    pub(crate) fn open(
        path: &PathBuf,
        preamble: &[u8],
    ) -> std::io::Result<Self> { /* … */ }

    /// Buffer one msgpack-serialised record.
    pub(crate) fn write<T: Serialize>(&mut self, record: &T) { /* … */ }

    /// Flush buffered records and patch the header record-count.
    pub(crate) fn finish(&mut self) -> std::io::Result<()> { /* … */ }
}

// Drop impl unchanged.
```

Removed from `src/trace.rs`:
- `TraceContext`, `MatTrace`, `TracePoint` structs.
- `write_geometry`, `write_polygon`, `write_mat`, `write_toolpath`,
  `write_toolpath_block` functions.

These move (B2) to the adaptive op recorder.

## B2. Move adaptive-specific block writers into `ops/assembly/adaptive/trace.rs`

`src/ops/assembly/adaptive/trace.rs` already owns the adaptive
`TraceRecord`. After B2 it also owns:

- `AdaptiveTraceContext` (renamed from `TraceContext`) — exact same shape.
- `MatTrace` — exact same shape, moved verbatim.
- `TracePoint` — moved verbatim.
- The four block writers (`write_geometry`, `write_polygon`, `write_mat`, `write_toolpath_block`) — moved verbatim into a `pub(super)` helper module or directly into `trace.rs`.
- A new `pub(super) fn open_adaptive(path, ctx) -> Option<Tracer>` that
  builds the preamble bytes (geometry + MAT blocks) and calls
  `Tracer::open(path, &preamble)`.
- A new `pub(super) fn finalize(tracer, toolpath)` that writes the
  toolpath block via the moved writer and then calls `tracer.finish()`.

The recorder's `TraceRecorder` API (`record_init`, `record_cut`,
`record_resume`, `record_exit`, `write_mat`, `finish`) keeps the same
signatures. The record struct keeps the same 28 fields. The only thing
that changes externally is who owns the binary-block writers.

## B3. Verify on-disk format unchanged

The point of Phase B is a pure code move — no format change. Verify
this:

1. Before starting B1, generate a reference trace file:
   ```
   raygeo trace /tmp/before.bin --scenario <default>
   ```
2. After B2, generate another:
   ```
   raygeo trace /tmp/after.bin --scenario <default>
   ```
3. The two files must be byte-identical:
   ```
   cmp /tmp/before.bin /tmp/after.bin
   ```
   (Adaptive clearing is deterministic per `cnc-module.md`'s
   determinism contract — same input → same bytes.)

If `make stubs` reports any Python surface change, the move has
escaped its scope. The Python bindings (`raygeo.ops.cut.interp.`,
`raygeo.ops.assembly.adaptive.*` paths and signatures) must be
untouched.

## B4. Verify

```
make format && make lint && make stubs && make docs && make test
```

Then manual round-trip:

```
raygeo trace /tmp/after.bin --scenario <default>
raygeo print /tmp/after.bin | head -50
raygeo inspect /tmp/after.bin
```

Toolpath + record stream render identically to before. No Python
imports changed.

---

# Phase C (optional follow-up — not part of this PR)

Once a second op (e.g. an `adaptive_entry` recorder, or `cnc/machining/hsm.rs`
from `cnc-module.md` Phase E) needs to emit trace files, do:

1. Introduce `OpType` enum + `CncTraceRecord` wrapper (common typed
   fields + msgpack `payload: Vec<u8>` for op-specific data).
2. Bump the magic byte to `b"ADP2"` and add a `version: u8` field so
   the Python reader can reject unknown ops with a clear error.
3. Make `python/raygeo/cli/trace.py` dispatch on `op_type` and decode
   per-op payloads via a registry keyed on `OpType`.

All of this is deferred *until a second op exists*. Phase B alone is
sufficient to make that next step cheap:

- `crate::trace` is now a generic container, so a new op recorder can
  plug in its own preamble (geometry / state / whatever) without
  touching the binary-writer code.
- The adaptive recorder's boundary with the container is now visible
  in code — `open_adaptive` / `finalize` — and is the template a new
  op would copy.

## Migration notes for Python consumers

There are **no** Python-facing breaking changes in this refactor:

- `raygeo.ops.cut.*` paths are unchanged.
- `raygeo.ops.assembly.adaptive.*` paths are unchanged.
- `raygeo.ops.assembly.entry.adaptive_entry` and
  `raygeo.ops.assembly.wavefront.adaptive_wavefronts` paths are
  unchanged.
- `raygeo trace / raygeo print / raygeo inspect` behaviour and file
  format are unchanged.

`migration.md` does not need an entry.

## Out of scope (tracked elsewhere, not by this refactor)

- The `cnc/` layer (`cnc/tool/`, `cnc/state_strategy.rs`,
  `cnc/machining/hsm.rs`) — `cnc-module.md` Phases D and E.
- Splitting or renaming `ops/cut/`. **Verified by audit:** most of it
  is motion-aware primitives, correctly located in `ops/`. The only
  pure-geometry candidate (`point_in_valid_area`) moves in Phase A.
- Softening the "ops must produce `Ops`" rule in `cnc-module.md` — the
  rule is imperfectly satisfied by `ops/cut/`, but that is a wording
  concern, not a code-relocation concern. May be addressed in a
  documentation PR.
- Per-op trace dispatch in the Python reader (Phase C above) —
  explicit non-goal until a second op materialises.

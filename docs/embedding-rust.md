# Embedding raygeo from Rust

raygeo's CNC stack is pure Rust (the Python bindings are optional), so a Rust
application can consume it as a plain cargo dependency:

```toml
raygeo = "1.68"
```

The default build is the pure-Rust core; the Python bindings are opt-in via
the `python` feature. A full working program is
[`examples/plan_clearing.rs`](../examples/plan_clearing.rs) — start there.

## The three steps

```rust,ignore
use raygeo::cnc::execution::run::run_plan;
use raygeo::cnc::plan::clearing::{ClearingWorkplanOptions, plan_clearing};

// 1. The pocket: boundary ring plus island rings (millimetres).
let part = Part::from_polygons(&boundary, &islands, (width, height));

// 2. The plan: regions, entries, passage crossings, wall finishing.
let plan = plan_clearing(&part, "", &options)?;

// 3. The toolpath: linked ops with safe-Z travels between steps.
let output = run_plan(&plan, &part)?;   // PlanOutput { ops, time_estimate }
```

`output.ops` is a normal `raygeo::ops::Ops` — hand it to your encoder, your
simulator, or your display. Execution has exactly one semantics in the stack:
Python's `run_intent` and Rust's `run_plan` delegate to the same core, so a
failing step fails the run with that step's error, on either surface.

## Feeds and spindle

`ClearingWorkplanOptions.feed_rate` (mm/min) puts a `SET_FEED_RATE` into the
emitted moves, so downstream G-code generation carries it. Without it the ops
stay feed-free, for embedders that map feeds themselves. Plunge moves are
distinguished at the ops level from cut moves by being vertical.

## Wall margin and stock to leave

`wall_margin` is honoured by every pass — entries, clearing, passage
crossings and the wall finishing profile (`finishing: true`): the tool stays
`tool_radius + wall_margin` from the walls and islands. `stock_to_leave` is
the finishing profile's extra material to leave for a later pass.

## Depth levels

A plan clears one depth in one call. For a pocket deeper than the step-down
you want, call `plan_clearing` + `run_plan` once per level with a different
`target_z` and concatenate the ops. Each level re-enters from `safe_z`, which
is safe: the passes at the level above are already in the ops you keep.

## Rest machining

To clear only what a previous tool left, seed the cleared area with the
previous tool's footprint:

```rust,ignore
let part = Part::from_polygons_initial(&boundary, &islands, &already_cleared, (w, h));
```

`already_cleared` is a list of polygons — the swept discs of the previous
tool's path. The clearing engine then treats those areas as gone.

## Multi-pocket parts

`Part::from_geometry_multi_face` splits a multi-pocket boundary into faces
(`""`, `"1"`, `"2"`, …). `plan_clearing` plans one face per call: loop the
faces, call it with the face id, and run each plan separately
(`run_plan` per face), concatenating the ops. Cleared-state threading between
steps restores into the step's own face, so keep each face's rest-machining
state separate (`Part::face_mut(id).cleared`).

## wasm32

The core builds for `wasm32-unknown-unknown`. The C/C++ parts (clipper2,
zstd) cross-compile with the WASI SDK toolchain targeting `wasm32-wasip1`,
whose objects are link-compatible with `wasm32-unknown-unknown` rustc output —
the same recipe this repository's CI uses (`.github/workflows/ci.yml`,
"Check wasm32") and the clipper2 repository's `scripts/wasm-check.sh`.

## Troubleshooting

- **Engine tracing in your test output**: dev builds print the engine's
  per-step tracing to stderr only when `RAYGEO_DEBUG=1`; release builds never
  print.
- **`a clearing step failed: resume point not found`**: the clearing pass
  could not find valid material to expand into — typically a pocket whose
  narrowest gap is at or below twice the tool radius. Widen the geometry,
  use a smaller tool, or let the planner's slot handling deal with the
  passage separately.
- **Walls left with a little material**: the adaptive pass keeps the tool
  centre `tool_radius (+ wall_margin)` inside an approximated region, so up
  to the region tolerance of material can remain on the wall; run a
  finishing profile pass (`finishing: true`) to bring the wall to size.

# Embedding raygeo from Rust

raygeo's CNC stack is pure Rust (the Python bindings are optional), so a Rust
application can consume it as a plain cargo dependency:

```toml
raygeo = { version = "1.68", default-features = false }
```

`default-features = false` keeps out the Python bindings and the allocator
override; the core (geometry, ops, cnc, pipeline) has no other prerequisites.
A full working program is `examples/plan_clearing.rs` — start there.

## The three steps

```rust,ignore
use raygeo::cnc::plan::clearing::{ClearingWorkplanOptions, plan_clearing};
use raygeo::cnc::execution::run::run_plan;

// 1. The pocket: boundary ring plus island rings (millimetres).
let part = Part::from_polygons(&boundary, &islands, (width, height));

// 2. The plan: regions, entries, passage crossings, wall finishing.
let plan = plan_clearing(&part, "", &options)?;

// 3. The toolpath: linked ops with safe-Z travels between steps.
let output = run_plan(&plan, &part)?;   // PlanOutput { ops, time_estimate }
```

`output.ops` is a normal `raygeo::ops::Ops` — feed it to your encoder, your
simulator, or your display. `feed_rate` in the options puts a `SET_FEED_RATE`
into the ops so downstream G-code generation carries it.

## Islands

Islands are respected everywhere: the adaptive clearing insets its valid
region by the tool radius around boundary *and* islands, and the wall-finishing
profile grows each island by the same clearance. No embedder-side repair is
needed; a position closer than the tool radius to an island is a bug.

## Depth levels

A plan clears one depth in one call. For a pocket deeper than one pass, call
`plan_clearing` + `run_plan` once per level with a different `target_z` and
concatenate the ops. Each level starts from fresh stock at that depth.

## Rest machining

To clear only what a previous tool left, seed the cleared area with the
previous tool's footprint:

```rust,ignore
let part = Part::from_polygons_initial(&boundary, &islands, &already_cleared, (w, h));
```

`already_cleared` is a list of polygons (the swept discs of the previous
tool's path). The clearing engine then treats those areas as gone.

## Multi-pocket parts

`Part::from_geometry_multi_face` splits a multi-pocket boundary into faces
(`""`, `"1"`, `"2"`, …). `plan_clearing` plans one face per call: loop the
faces, call it with the face id, and merge the resulting steps with
face-prefixed node keys if you share one pipeline run — or simply run one
pipeline per face and concatenate the ops. Note that cleared-state threading
between steps restores into the step's face only, so keep each face's rest
machining state separate (`Part::face_mut(id).cleared`).

## wasm32

The core builds for `wasm32-unknown-unknown`. The C/C++ parts (clipper2,
zstd) cross-compile with the WASI SDK toolchain targeting `wasm32-wasip1`,
whose objects are link-compatible with `wasm32-unknown-unknown` rustc output —
the same recipe this repository's CI uses (see `.github/workflows/ci.yml`,
"Check wasm32"), and the clipper2 repository's `scripts/wasm-check.sh`.

## Troubleshooting

- **Engine tracing output**: dev builds print per-step tracing to stderr only
  when `RAYGEO_DEBUG=1`; release builds never print.
- **`a clearing step failed: resume point not found`**: the clearing pass
  could not find valid material to expand into — typically a pocket whose
  narrowest gap is at or below twice the tool radius. Widen the geometry, use
  a smaller tool, or leave the passage to a separate slot operation.
- **Clearing leaves the wall un-cut**: the adaptive pass keeps the tool radius
  (plus `wall_margin`, once honoured) inside the boundary; run a finishing
  profile pass (`finishing: true` in the options, or a separate
  `ProfileSpec`) to bring the wall to size.

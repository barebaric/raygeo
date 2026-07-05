# Refactor: `adaptive_peeling` → layered `geo`/`ops`/`cnc` architecture

## Problem

`adaptive_peeling` (in `src/geo/algo/hsm.rs`) currently:

1. Runs a geometric iteration loop (bite → extract cutting arc → incorporate).
2. Links cutting arcs into a tour with MAT-routed travel between them.
3. Encodes cut-vs-travel as Z height in a flat `Vec<Point3D>`.

Step 3 leaks CNC-domain decisions (which segments are cutting vs rapid)
into the geometry layer. Step 2 (tour assembly) is motion assembly, not
pure geometry. Both belong above `geo`.

## Architectural rules

```
geo  →  ops  →  cnc      (never import upward)
```

| Layer | Owns                                                                                                                                                                      | Does NOT know about                  |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| `geo` | Primitives & pure geometric algorithms. No "what to cut" strategy decisions, no tour, no motion verbs, no `Ops`.                                                          | machining, motion, tools, `Ops`      |
| `ops` | Motion assembly: clearing strategies, linking, classification, and Ops emission. **All assemblers produce and consume `Ops` objects — never raw polygon/polyline lists.** | tools, spindle RPM, feed-rate values |
| `cnc` | Operation orchestration: sequences entry + clear + finish, resolves tool-aware `State` via `StateStrategy`, drives `geo`/`ops` primitives.                                | geometry algorithms                  |

### Key constraint

Once in the ops layer, assemblers **must provide and operate on `Ops`**.

- Public ops-layer functions return `Ops` (not `MotionPath`, not
  `Vec<Vec<Point>>`, not `Vec<Point3D>`).
- Public ops-layer composition functions accept `Ops` inputs.
- Intermediate types (`MotionPath`, `MotionRole`) may exist as
  `pub(crate)` implementation details but are never part of the public
  ops API.
- Motion classification is encoded as `MoveTo` (rapid/travel) vs
  `LineTo` (feed/cut) at the `Ops` command level — the existing
  command-type distinction. No parallel classification enum leaks out.
- `State` values (feed_rate, spindle_rpm, coolant, …) are passed in by
  the caller (cnc layer). Ops assemblers apply them via a new
  `Ops::apply_state(&State)` helper but never compute them.

### Testing stance

**All tests are Python.** No Rust `#[cfg(test)]` modules. Every new
Rust method or function that needs testing must be exposed through the
PyO3 Python binding layer so it can be tested via `pytest`. This
follows the fcadapt `cnc.md` stance.

### Backward compatibility

No backward compatibility should be added, no deprecations, etc. - clean port that removes any old APIs. Backward compatibility does not matter.
Breaking changes should however be documented in migration.md.


## Layer boundary — final mapping

### `src/geo/` (primitives only)

**Stays:**

- `find_cutting_arc`, `fillet_arc_ends`, `find_safe_sweep_end`,
  `quarter_fillet`, `build_fillet_candidate` — pure geometric utilities.
- `ClearedArea` with `bites`, `bite_in_direction`, `incorporate`,
  `expand`, `expand_step`, etc.
- `compute_medial_axis`, `mat_path`, `smooth_path`,
  `compute_inset_region`, `offset_polygon_with_style`, all
  polygon/line/arc shape ops.

**Removed (moved to ops):**

- `adaptive_peeling` → `ops::adaptive_peeling`
- `link_filleted_arcs` → `ops::link_arcs_to_ops`
- `link_cutting_arcs` → folded into `adaptive_peeling`
- `adaptive_entry` → `ops::adaptive_entry` (returns `Ops`)
- `adaptive_wavefronts` → `ops::adaptive_wavefronts` (returns `Ops`)

**Added:**

- `ClearedArea::remaining_in_inset(boundary, obstacles, radius)`
  → `Vec<Polygon>` — computes inset region, exposed to Python.

### `src/ops/` (motion assembly — Ops-centric)

**Restructured into `assembly/` submodule:**

- `assembly/polyline.rs` (moved from `ops/polyline.rs`)
- `assembly/lead_in_out.rs` (moved)
- `assembly/overscan.rs` (moved)
- `assembly/raster/` (moved)
- `assembly/tabs.rs` (moved)

**`assembly/hsm.rs` — all HSM motion assembly:**

```rust
// ── Entry strategy ────────────────────────────────────

pub struct AdaptiveEntryOptions { ... }
pub struct AdaptiveEntryResult { pub ops: Ops, pub cleared_polygons: Vec<Polygon> }

pub fn adaptive_entry(
    opts: &AdaptiveEntryOptions, cut_state: &State,
) -> AdaptiveEntryResult;

// ── Wavefront expansion ───────────────────────────────

pub struct AdaptiveWavefrontOptions { ... }

pub fn adaptive_wavefronts(
    cleared: &mut ClearedArea,
    opts: &AdaptiveWavefrontOptions,
    cut_state: &State,
) -> Ops;

// ── Peeling (D-cut) clearing ──────────────────────────

/// Run the peeling (D-cut) clearing strategy and return an
/// Ops with cutting arcs as LineTo (at cut_z) and travel
/// links as MoveTo (at safe_z). States are applied per
/// motion role.
pub fn adaptive_peeling(
    cleared: &mut ClearedArea,
    pocket_boundary: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    step_over: f64,
    cut_z: f64,
    safe_z: f64,
    wall_margin: f64,
    travel_smoothing: i32,
    area_tolerance: f64,
    cut_state: &State,
    travel_state: &State,
) -> Ops;

// ── Arc linking ───────────────────────────────────────

/// Link pre-computed arcs into an Ops with MAT-routed
/// travel. Exposed publicly so the cnc layer can link
/// finishing-pass offsets the same way.
pub fn link_arcs_to_ops(
    arcs: &[Vec<Point>],
    uncleared: &[Polygon],
    mat: Option<&MedialAxis>,
    cut_z: f64,
    safe_z: f64,
    safe_margin: f64,
    smoothing_amount: i32,
    preserve_order: bool,
    cut_state: &State,
    travel_state: &State,
) -> Ops;
```

**New method on `Ops` (`src/ops/container.rs`):**

```rust
impl Ops {
    /// Emit StateCmd nodes for every field of `state` that is set.
    /// Domain-neutral: does not decide what values to use, just
    /// emits them. The caller (cnc layer) computes the State.
    pub fn apply_state(&mut self, state: &State) {
        // power is always emitted (default 0.0).
        self.set_power(state.power);
        if let Some(fr) = state.feed_rate { self.set_feed_rate(fr); }
        if let Some(rr) = state.rapid_rate { self.set_rapid_rate(rr); }
        if let Some(rpm) = state.spindle_rpm { self.set_spindle_rpm(rpm); }
        if let Some(c) = state.coolant { self.set_coolant(c); }
        if let Some(f) = state.frequency { self.set_frequency(f); }
        if let Some(pw) = state.pulse_width { self.set_pulse_width(pw); }
        if let Some(ref h) = state.active_head_uid { self.set_head(h); }
    }
}
```

### `src/cnc/` (operation orchestration)

**Copied verbatim from `../raygeo-fcadapt/src/cnc/`:**

- `mod.rs` — module declarations + re-exports (adds `pub mod machining;`).
- `tool/{mod,shape,material}.rs` — `Tool`, `ToolShape`, `ToolMaterial`.
- `state_strategy.rs` — `StateStrategy`, `StateContext`, `MovePhase`.
- `entry.rs` — `EntryStrategy`.

**New hierarchical `machining/` submodule** — one file per operation type, no flat `clearing_hsm/`:

- `machining/mod.rs` — `pub mod hsm;`
- `machining/hsm.rs` — HSM adaptive clearing orchestrator (`HsmClearParams`, `adaptive_clear_hsm`)

**`src/cnc/machining/hsm.rs`:**

```rust
pub struct HsmClearParams {
    pub step_over: f64,
    pub target_z: f64,
    pub safe_z: f64,
    pub wall_margin: f64,
    pub travel_smoothing: i32,
    pub area_tolerance: f64,
    pub stock_to_leave: f64,
    pub finishing_pass: bool,
    pub state_strategy: StateStrategy,
    pub entry: EntryStrategy,
}

pub fn adaptive_clear_hsm(
    pocket_boundary: &Polygon,
    islands: &[Polygon],
    tool: &Tool,
    params: &HsmClearParams,
) -> Result<Ops, RaygeoError>;
```

Orchestrator body:

```rust
pub fn adaptive_clear_hsm(...) -> Result<Ops, RaygeoError> {
    let mut cleared = ClearedArea::new();
    let mut ops = Ops::new();

    // (1) Resolve states from Tool + StateStrategy
    let cut_state     = params.state_strategy.state_for(
        &StateContext { tool, phase: MovePhase::Cutting });
    let plunge_state  = params.state_strategy.state_for(
        &StateContext { tool, phase: MovePhase::Plunge });
    let travel_state  = params.state_strategy.state_for(
        &StateContext { tool, phase: MovePhase::Travel });
    let retract_state = params.state_strategy.state_for(
        &StateContext { tool, phase: MovePhase::Retract });

    // (2) Preamble: spindle + coolant
    ops.apply_state(&cut_state);

    // (3) Entry (geo) → Ops (ops::polyline_to_ops)
    let entry_opts = entry_opts_from(pocket_boundary, islands, tool, params);
    let entry = cnc::machining::hsm::adaptive_entry(&entry_opts);
    cleared.add_cleared_polygons(&entry.cleared_polygons);
    let entry_ops = polyline_to_ops(&entry.toolpath, true);
    ops.extend(&entry_ops);

    // (4) Peeling strategy + motion assembly (ops) — single call, returns Ops
    let peel_ops = plan_peeling_clear(
        &mut cleared, pocket_boundary, islands,
        tool.shape.diameter() / 2.0,
        params.step_over, params.target_z, params.safe_z,
        params.wall_margin, params.travel_smoothing,
        params.area_tolerance,
        &cut_state, &travel_state,
    );
    ops.extend(&peel_ops);

    // (5) Optional finishing pass (geo offset → ops linking)
    if params.finishing_pass {
        let inset_dist = -(tool.shape.diameter() / 2.0 + params.stock_to_leave);
        let finish_rings = offset_polygon_with_style(
            pocket_boundary, inset_dist, JoinStyle::Round);
        if !finish_rings.is_empty() {
            let remaining = cleared.remaining_in_inset(
                pocket_boundary, islands, tool.shape.diameter() / 2.0);
            let finish_ops = link_arcs_to_ops(
                &finish_rings, &remaining, None,
                params.target_z, params.safe_z,
                tool.shape.diameter() / 2.0, 0, true,
                &cut_state, &travel_state,
            );
            ops.extend(&finish_ops);
        }
    }

    // (6) Final retract
    let last = last_xy(&ops);
    ops.apply_state(&retract_state);
    ops.move_to(last.0, last.1, params.safe_z, None);

    Ok(ops)
}
```

---

## Implementation plan

Phases A–C are **complete**. Only D and E remain.

### Verification protocol (applies to every step)

| Check           | Command                    | When                                          |
| --------------- | -------------------------- | --------------------------------------------- |
| Format + lint   | `make format && make lint` | every step                                    |
| Python tests    | `make test`                | every step                                    |
| Stubs           | `make stubs`               | every step that adds/changes a Python binding |
| Docs + examples | `make docs`                | every step that adds/changes examples or API  |

**Verify (full):** `make format && make check && make stubs && make docs`

For steps with no Python API change (pure file moves), run the same
commands and verify stubs/docs diffs are empty.

---

### Phase D — CNC scaffolding (non-breaking additions)

#### Step D1: Copy cnc module + Python bindings + test + example + visual

**Rust files:**

- `src/cnc/mod.rs` (NEW) — adapted from fcadapt; add `pub mod machining;`.
- `src/cnc/machining/mod.rs` (NEW) — `pub mod hsm;`
- `src/cnc/machining/hsm.rs` (NEW) — `adaptive_clear_hsm` orchestrator + `HsmClearParams`.
- `src/cnc/tool/{mod.rs, shape.rs, material.rs}` (NEW) — copied verbatim from `../raygeo-fcadapt/src/cnc/`.
- `src/cnc/state_strategy.rs` (NEW) — copied verbatim.
- `src/cnc/entry.rs` (NEW) — copied verbatim.
- `src/lib.rs` — add `pub mod cnc;`.

**Python bindings:**

- `src/python/cnc/mod.rs` (NEW) — register submodule (includes `pub mod machining;`).
- `src/python/cnc/machining/mod.rs` (NEW) — `pub mod hsm;` registration.
- `src/python/cnc/machining/hsm.rs` (NEW) — `#[pyfunction]` wrapper returning `PyOps`.
- `src/python/cnc/tool/{mod.rs, shape.rs, material.rs}` (NEW) — copied from `../raygeo-fcadapt/src/python/cnc/`.
- `src/python/cnc/state_strategy.rs` (NEW) — copied.
- `src/python/cnc/entry.rs` (NEW) — copied.
- `src/python/mod.rs` — add `pub mod cnc;`.
- `src/lib.rs` — add `python::cnc::register(m)?;` in `raygeo()`.

**Python tests** (`tests/cnc/`, NEW):

- `tests/cnc/test_tool.py`:

```python
from raygeo.cnc import Tool, ToolShape, ToolMaterial

def test_tool_construction():
    t = Tool(label="6mm EM",
             shape=ToolShape.EndMill(diameter=6, shank_diameter=6,
                 cutting_edge_height=15, flute_count=3, overall_length=50),
             material=ToolMaterial.Carbide, stickout=15, coating=None)
    assert t.shape.diameter() == 6.0
    assert t.default_stickout() > 0
```

- `tests/cnc/test_state_strategy.py`:

```python
from raygeo.cnc import StateStrategy
from raygeo.cnc.state_strategy import MovePhase

def test_state_strategy_constant():
    s = StateStrategy.constant(feed_rate=1200, plunge_rate=300,
        rapid_rate=8000, spindle_rpm=18000, coolant="Flood")
    cut = s.state_for(MovePhase.Cutting)
    assert cut.feed_rate == 1200
    assert cut.spindle_rpm == 18000
    travel = s.state_for(MovePhase.Travel)
    assert travel.feed_rate is None
    assert travel.rapid_rate == 8000
```

- `tests/cnc/test_entry.py`:

```python
from raygeo.cnc import EntryStrategy
from raygeo.geo.algo.helix import HelixDirection

def test_entry_strategy_helix():
    e = EntryStrategy.helix(direction=HelixDirection.Ccw, pitch=2.0,
        target_diameter=5.0, min_diameter=1.5, expand_to_diameter=None)
    assert e is not None
```

**Image generator** (`tools/examples/cnc_tool.py`, NEW):

- `generate_tool_shapes()` — show EndMill / BallNose / BullNose with labeled dimensions.
- `generate_state_strategy()` — bar chart of feed/plunge/rapid/spindle for a `Constant` strategy.
- `__docs_target__ = ["raygeo.cnc.tool.md", "raygeo.cnc.state_strategy.md"]`

**Visual test page** (`tools/visual_test/cnc_tool.py`, NEW):

- `page_cnc_tool()` — construct a Tool interactively (shape dropdown, diameter slider) and show derived properties.
- Register in `tools/visual_test.py`: import + sidebar `"CNC Tool"` + dispatch.

**Format + lint:** `make format && make lint`

**Stubs:** `make stubs` — verify `python/raygeo/cnc/` stubs generated for tool, state_strategy, entry.

**Docs:** `make docs` — verify `docs/api/raygeo.cnc.*.md` pages generated with inline images.

**Verify (full):** `make format && make check && make stubs && make docs`

---

### Phase E — Orchestrator (the finale)

#### Step E1: `adaptive_clear_hsm` (Rust + Python binding + test + example + visual)

**Rust files:**

- `src/cnc/machining/hsm.rs` (NEW) — `adaptive_clear_hsm` + `HsmClearParams` in one file.
- `src/cnc/machining/mod.rs` — add `pub mod hsm;` and re-export.
- `src/cnc/mod.rs` — already has `pub mod machining;` from Phase D; no change needed.

**Python binding:**

- `src/python/cnc/machining/hsm.rs` (NEW) — `#[pyfunction]` wrapper returning `PyOps`.
- `src/python/cnc/machining/mod.rs` (NEW) — `pub mod hsm;` and register.
- `src/python/cnc/mod.rs` — already has `pub mod machining;` from Phase D.

**Python tests** (`tests/cnc/machining/test_hsm.py`, NEW):

```python
from raygeo.cnc.machining.hsm import (
    adaptive_clear_hsm, HsmClearParams,
)
from raygeo.cnc import Tool, ToolShape, ToolMaterial, StateStrategy, EntryStrategy

def _make_tool():
    return Tool(label="6mm EM",
        shape=ToolShape.EndMill(diameter=6, shank_diameter=6,
            cutting_edge_height=15, flute_count=3, overall_length=50),
        material=ToolMaterial.Carbide, stickout=15, coating=None)

def _make_params(finishing=True):
    return HsmClearParams(
        step_over=2, target_z=-5, safe_z=2,
        wall_margin=0, travel_smoothing=50, area_tolerance=1,
        stock_to_leave=0.2, finishing_pass=finishing,
        state_strategy=StateStrategy.constant(
            feed_rate=1200, plunge_rate=300, rapid_rate=8000,
            spindle_rpm=18000, coolant="Flood"),
        entry=EntryStrategy.helix(pitch=2, target_diameter=5, min_diameter=1.5),
    )

def test_adaptive_clear_rect():
    pocket = [(0,0), (40,0), (40,40), (0,40)]
    ops = adaptive_clear_hsm(regions=[pocket], tool=_make_tool(), params=_make_params())
    assert ops.len() > 0
    assert ops.cut_distance() > 0
    assert ops.travel_distance() > 0

def test_adaptive_clear_island():
    pocket = [(0,0), (50,0), (50,50), (0,50)]
    islands = [[(20,20),(30,20),(30,30),(20,30)]]
    ops = adaptive_clear_hsm(regions=[pocket], islands=islands,
                             tool=_make_tool(), params=_make_params())
    assert ops.len() > 0
    # No cutting endpoint inside island
    for i in range(ops.len()):
        if ops.is_cutting(i):
            ep = ops.endpoint(i)
            assert not (20 <= ep[0] <= 30 and 20 <= ep[1] <= 30)

def test_adaptive_clear_determinism():
    pocket = [(0,0), (40,0), (40,40), (0,40)]
    ops1 = adaptive_clear_hsm(regions=[pocket], tool=_make_tool(), params=_make_params())
    ops2 = adaptive_clear_hsm(regions=[pocket], tool=_make_tool(), params=_make_params())
    assert ops1.format_dump() == ops2.format_dump()

def test_adaptive_clear_has_spindle():
    pocket = [(0,0), (40,0), (40,40), (0,40)]
    ops = adaptive_clear_hsm(regions=[pocket], tool=_make_tool(), params=_make_params())
    types = [ops.command_type(i) for i in range(ops.len())]
    assert CommandType.SET_SPINDLE_RPM in types
    assert CommandType.SET_COOLANT in types
```

**Image generator** (`tools/examples/cnc_machining_hsm.py`, NEW):

- `generate_clear_rect_2d()` — rectangular pocket; 2D plot with cutting colored by Z, travel dashed orange, boundary black.
- `generate_clear_rect_3d()` — same in 3D with Z axis.
- `generate_clear_island_2d()` — pocket with island; island in red.
- `generate_clear_with_finish()` — clearing + finishing pass in different colors.
- `__docs_target__ = ["raygeo.cnc.machining.hsm.md"]`
- `__images__` list with heading `"adaptive_clear_hsm"`.

**Visual test page** (`tools/visual_test/cnc_machining_hsm.py`, NEW):

- `page_cnc_machining_hsm()` — presets (rectangle, rectangle+island, L-shape); sliders (tool diameter, step_over, target_z, safe_z); checkboxes (finishing_pass, show_travel, show_3d); renders Ops.
- Register in `tools/visual_test.py`: import + sidebar `"CNC Machining HSM"` + dispatch.

**Format + lint:** `make format && make lint`

**Stubs:** `make stubs` — verify `python/raygeo/cnc/machining/hsm/` stubs generated.

**Docs:** `make docs` — verify `docs/api/raygeo.cnc.machining.hsm.md` with inline images.

**Verify (full):** `make format && make check && make stubs && make docs`

---

#### Step E2: Full validation

**All files** — final pass.

**Checklist:**

- [ ] `make format` — auto-format all Rust + Python (including PEP8 import ordering).
- [ ] `make lint` — cargo fmt check + clippy + ruff + pyright, all clean.
- [ ] `make test` — full pytest suite passes.
- [ ] `make stubs` — stubs regenerated cleanly.
- [ ] `make docs` — docs regenerated with all inline images.
- [ ] `make visual` — streamlit app launches; all pages render without errors (spot-check: "HSM Assembly", "CNC Tool", "CNC Machining HSM").
- [ ] Determinism: run `adaptive_clear_hsm` twice on the same input, verify byte-identical output (`test_adaptive_clear_determinism`).
- [ ] No lingering references to removed `adaptive_peeling` / `link_filleted_arcs` anywhere in `src/`, `tests/`, `tools/`, `python/` (verify with `rg adaptive_peeling src/ tests/ tools/ python/`).
- [ ] No remaining `clearing_hsm` references in `src/`, `tests/`, `tools/`, or `python/` (verify with `rg clearing_hsm src/ tests/ tools/ python/`).

**Verify (full):** `make format && make check && make stubs && make docs`

---

## Testing strategy

### Test placement

| Layer | Python tests                        | Image generator                | Visual test page                  |
| ----- | ----------------------------------- | ------------------------------ | --------------------------------- |
| `geo` | `tests/geo/algo/test_geo_algo_*.py` | `tools/examples/geo_algo_*.py` | `tools/visual_test/geo_algo_*.py` |
| `ops` | `tests/ops/test_*.py`               | `tools/examples/ops_*.py`      | `tools/visual_test/ops_*.py`      |
| `cnc` | `tests/cnc/**/test_*.py`            | `tools/examples/cnc_*.py`      | `tools/visual_test/cnc_*.py`      |

**No Rust tests.** All tests go through the PyO3-exposed Python API.
Every new Rust function or method that needs testing must be exposed
through a Python binding. Internal helpers that aren't worth exposing
are tested indirectly via the public API that calls them.

### Image generators (`tools/examples/`)

Each example module follows this contract:

```python
__docs_target__ = ["raygeo.ops.assembly_hsm.md"]  # API doc pages to inject images into

__images__ = [
    {
        "heading": "plan_peeling_clear",   # ### heading in the API doc, or None for top of page
        "caption": "...",
        "function": generate_plan_peeling,  # returns a matplotlib Figure
    },
    ...
]
```

Functions named `generate_*` return a `matplotlib.figure.Figure`. The
CLI (`python -m tools.cli all`) discovers all modules in
`tools/examples/`, calls each `generate_*`, saves the figure to
`docs/api/images/`, and injects it into the API doc page(s) listed in
`__docs_target__` under the matching `### heading`.

Run: `make docs` (or `python -m tools.cli examples` for images only).

### Visual test pages (`tools/visual_test/`)

Each page is a streamlit module with a `page_*()` function. Registration
requires three edits to `tools/visual_test.py`:

1. Import: `from tools.visual_test.<module> import page_<name>`
2. Sidebar: add the page label to the `st.sidebar.radio(...)` list.
3. Dispatch: add `elif page == "<label>": page_<name>()`.

Run: `make visual` (launches streamlit at `localhost:8501`).

### Verification commands

| Command       | What it does                                                                      |
| ------------- | --------------------------------------------------------------------------------- |
| `make format` | Auto-format all Rust (`cargo fmt`) + Python (`ruff format` + `ruff check --fix`). |
| `make lint`   | Check formatting + clippy + ruff + pyright. Fails on any issue.                   |
| `make test`   | `pytest -v` — all Python tests.                                                   |
| `make check`  | `make lint && make test`.                                                         |
| `make stubs`  | Regenerate `.pyi` stub files (`cargo run --bin stub_gen`).                        |
| `make docs`   | Regenerate API docs + visual example images + inject into API pages.              |
| `make visual` | Launch streamlit visual test playground.                                          |

### Determinism contract

For identical inputs (`pocket_boundary`, `islands`, `tool`, `params`),
`adaptive_clear_hsm` MUST produce byte-identical `Ops` across runs. This
is tested in `test_adaptive_clear_determinism` (Step E1) and must be
maintained in all future changes.

### Regression fixtures

The visual examples under `tools/examples/` double as regression
fixtures. After Phase E, `tools/examples/cnc_machining_hsm.py` renders
the canonical pocket-with-island case. Any algorithm change that alters
the output should be visible in the rendered image.

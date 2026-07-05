# Available commands

- `make dev` — build and install into the active venv
- `make stubs` — re-generate `.pyi` type stubs (after changing `src/python/` bindings)
- `make lint` — lint all code (Rust + Python)
- `make lint-rust` — lint Rust only
- `make lint-python` — lint Python only
- `make format` — auto-format all code (Rust + Python, including PEP8 import ordering)
- `make format-rust` — format Rust only
- `make format-python` — format Python only (ruff handles formatting and import sorting)
- `make test` — run the full test suite. make sure to "make dev" before you test
- `make check` — lint + test
- `make docs` — re-generate the API docs
- `make build` — build the wheel (release)

# Adaptive clearing tracing

- `raygeo trace <path>` — run adaptive clearing with tracing, write `.bin` file
- `raygeo print <path>` — dump all trace records as grep-friendly lines
- `raygeo inspect <path>` — interactive matplotlib viewer
- `raygeo profile` — profile adaptive clearing performance
- Optional flags: `--scenario`, `--svg`, `--tool-radius`, `--advance`, `--step-over`

# Rules

- You are strictly forbidden from editing stubs manually. They are only to be edited using "make stubs".
- You should never edit markdown docs. They are auto-generated.
- Use make commands when available - avoid calling the underlying tools directly.

# Layering Rules Specification

The crate is split into three layers that depend only downward:
`geo` → `ops` → `cnc`. Never import upward.

| Layer | Owns                                                                                                                       | Does NOT know                   |
| ----- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| `geo` | Primitives & pure geometric algorithms. No "what-to-cut" decisions, no motion verbs, no machining terminology, no `Ops`.   | machining, motion, tools, `Ops` |
| `ops` | Motion assembly: clearing strategies, linking, classification, and Ops emission. All assemblers produce and consume `Ops`. | tools, spindle, feed values     |
| `cnc` | Operation orchestration: sequences entry + clear + finish.                                                                 | geometry algorithms             |

# Adaptive clearing: resume and routing

When the stepper loses engagement, the adaptive clearing loop runs a two-phase
recovery:

## Phase 1 — Resume strategies (`resume.rs`)

Find _where_ to resume cutting (a target point on the uncleared boundary).

Each strategy returns `(source, resume_point)` or `None`. They are tried in
priority order: WallHug → Segment → MAT → Frontier → Envelope → Island.
Per-strategy outcomes are recorded in `resume_strategy_reasons[0..5]` and
`resume_strategy_details[0..5]` in the trace record.

## Phase 2 — Routing strategies (`routing.rs`)

Find _how_ to travel from the tool's current position to the resume point
without colliding with uncleared material or islands.

Each strategy returns `(source, smoothed_waypoints)` or `None`. Tried in
priority order: Direct → Frontier → MAT → AStar. Per-strategy outcomes are in
`route_strategy_details[0..3]`.

## Contract

- Resume strategies must NOT perform routing. They only select a target.
- The resume point returned by a resume strategy MUST lie in fully cleared
  area, and probing from it MUST yield a successful result (non-zero
  engagement).
- Routing strategies must NOT select the target. They only find a safe path
  between two given points.
- On routing failure, the candidate is blacklisted and phase 1 retries with
  the next strategy.
- Phase 1 and phase 2 failures are independent and independently observable in
  traces (`strat=` vs `rout=` fields).

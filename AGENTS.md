# Available commands

- `make build` — build the wheel (release)
- `make dev` — build and install into the active venv
- `make stubs` — re-generate `.pyi` type stubs (after changing `src/python/` bindings)
- `make lint` — lint all code (Rust + Python)
- `make lint-rust` — lint Rust only
- `make lint-python` — lint Python only
- `make format` — auto-format all code (Rust + Python, including PEP8 import ordering)
- `make format-rust` — format Rust only
- `make format-python` — format Python only (ruff handles formatting and import sorting)
- `make test` — run the full test suite
- `make check` — lint + test
- `make docs` — re-generate the API docs

# Adaptive clearing tracing

- `python tools/adaptive_inspector.py trace <path>` — run adaptive clearing with tracing, write `.bin` file
- `python tools/adaptive_inspector.py print <path>` — dump all trace records as grep-friendly lines
- `python tools/adaptive_inspector.py inspect <path>` — interactive matplotlib viewer
- Optional flags: `--scenario`, `--svg`, `--tool-radius`, `--advance`, `--step-over`

# Rules

- You are strictly forbidden from editing stubs manually. They are only to be edited using "make stubs".
- You should never edit markdown docs. They are auto-generated.
- Use make commands when available - avoid calling the underlying tools directly.

# Layering Rules Specification

The crate is split into three layers that depend only downward:
`geo` → `ops` → `cnc`. Never import upward.

- `src/geo/` — pure geometry. Points, paths, offsets, algorithms. Knows
  nothing about machining, motion commands, or tools.
- `src/ops/` — the `Ops` command container and domain-neutral motion assembly
  (raster, lead-in/out, polyline, …). Holds the generic `State` representation
  (`feed_rate`, `spindle_rpm`, `coolant`, … as optional fields) so `Ops` can
  carry any machine's state, but contains no domain logic that fills those
  fields.
- `src/cnc/` — the CNC domain: Operation orchestration: sequences operations
  (e.g. entry + clear + finish), resolves tool-aware `State` via `StateStrategy`,
  drives `geo`/`ops` primitives.

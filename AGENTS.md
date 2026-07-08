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
- `python -m tools.cli doc ops.feature.slot` - run example plot generator

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

## Contract

- Resume strategies must NOT perform routing. They only select a target.
- The resume point returned by a resume strategy MUST lie in fully cleared
  area, and probing from it MUST yield a successful result (non-zero
  engagement).
- Routing strategies must NOT select the target. They only find a safe path
  between two given points.
- On routing failure, the candidate is blacklisted and phase 1 retries with
  the next strategy.

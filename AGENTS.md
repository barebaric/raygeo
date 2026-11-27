# Available commands

- `make dev` — build and install into the active venv
- `make stubs` — re-generate `.pyi` type stubs (after changing `src/python/` bindings)
- `make lint` — lint all code (Rust + Python)
- `make lint-rust` — lint Rust only
- `make lint-python` — lint Python only
- `make format` — auto-format all code (Rust + Python, including PEP8 import ordering)
- `make format-rust` — format Rust only
- `make format-python` — format Python only (ruff handles formatting and import sorting)
- `make test` — fast test lane (no slow/stress/hardware). Make sure to "make dev" before you test
- `make test-all` — full suite, including slow and stress tests
- `make test-geometry` / `test-engine` / `test-encode` / `test-driver` — area lanes
- `make test-slow` — slow and stress tests only
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
- Never add Rust tests (`#[cfg(test)]` / `#[test]` blocks in `src/`).
  All tests are Python-based, under `tests/`. Exercise new Rust code
  through PyO3 bindings from Python test code.

# Layering Rules Specification

The crate is split into five layers. Four are domain layers that
depend only downward (`geo` → `ops` → `cnc`, and `geo` → `sketch`);
`pipeline` is a generic, domain-free runtime that `cnc` depends on
(it sits below `cnc`, alongside `ops`). Total dependency order:

```
geo → ops \
            → cnc
geo → sketch
pipeline -/

geo → formats   (leaf: no other layer imports it)
driver          (leaf: transports and protocol sessions; no ops imports)
```

Never import upward.

| Layer      | Owns                                                                                                                      | Does NOT know                                 |
| ---------- | ------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `geo`      | Primitives & pure geometric algorithms. No "what-to-cut" decisions, no motion verbs, no machining terminology, no `Ops`.  | machining, motion, tools, `Ops`, `pipeline`   |
| `sketch`   | Sketch constraint solving, entity model and topology; depends only on `geo` primitives.                                   | machining, motion, tools, `Ops`, `pipeline`   |
| `ops`      | CNC and laser building blocks: clearing strategies, linking, classification, Ops emission, and Ops-to-format converters (`ops::convert`, including the G-code, Epilog and Ruida machine-code encoders). | plans, intents, `pipeline` runtime, device transports/sessions |
| `pipeline` | Generic runtime to execute machining agnostic intents in a rayon thread pool.                                             | geometry, machining, `Ops`, tools, any domain |
| `cnc`      | Plan-time orchestration: build descriptive `Plan`s (sequences of `PlanStep`s) and convert them into executable `Intent`s. | geometry algorithms                           |
| `formats`  | File-format import and export (`coreldraw`, `svg`, `gcode`, `dxf`); depends only on `geo`, sibling format modules and the shared error type. | `ops`, `cnc`, `sketch`, `pipeline`, machining |
| `driver`   | Device transports and protocol sessions (GRBL, Marlin, Smoothie, ESP3D, OctoPrint, xTool, Epilog, Ruida, simulator, remote daemon). Pure Rust; Python bindings under `raygeo.driver`. | `ops`, `cnc`, `pipeline`, geometry algorithms, encoders |

Machine-code encoders are converters (`Ops` + caller-supplied dialect/job
config -> text or bytes), not drivers. They live in `ops::convert`; device
transports, protocol sessions, flow control and reconnect logic live in
`src/driver` and are exposed as `raygeo.driver.*`.

Driver rules:
- `src/driver` is pure Rust and must not depend on PyO3; the `python`
  feature adds the bindings under `src/python/driver`.
- Protocol modules may use `src/driver/core` freely. Cross-protocol
  imports need an explicit justification in the importing module.
- Keep the GRBL protocol byte-for-byte compatible with Rayforge's Python
  driver; do not change protocol quirks without an explicit request.
- Driver emulators (`python/raygeo/driver/*/emulator.py`) are real
  firmware/server models used by tests, not mocks.

# File Formats

Readers and writers for external file formats live under `formats`, one
module per format, with the same name at every level:

- Rust core: `src/formats/<format>/`
- PyO3 bindings: `src/python/formats/<format>/`, registered in
  `src/python/formats/mod.rs` as `raygeo.formats.<format>`
- Tests: `tests/formats/<format>/test_<format>*.py`, fixtures in
  `tests/formats/<format>/assets/`
- Example generators: `tools/examples/formats_<format>*.py`
- API docs are generated from the stubs as `docs/api/raygeo.formats.<format>.md`

# Export Policy: Explicit Paths

Every item has exactly one canonical path — its leaf module. Parent mod.rs
must not re-export children's items (no namespace flattening).

Exceptions: Primitive types, errors, classes and constants that are
_public_ AND _shared_ within a submodule, such as an AssemblyResult.

Python sub-modules mirror the Rust hierarchy - no aliases or re-exports
at higher levels.

## Contract

- Resume strategies must NOT perform routing. They only select a target.
- The resume point returned by a resume strategy MUST lie in fully cleared
  area, and probing from it MUST yield a successful result (non-zero
  engagement).
- Routing strategies must NOT select the target. They only find a safe path
  between two given points.
- On routing failure, the candidate is blacklisted and phase 1 retries with
  the next strategy.

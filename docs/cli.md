## CLI Tools

raygeo ships a `raygeo` CLI for adaptive clearing development and debugging.
Install with the optional `cli` dependency group:

```bash
pip install -e ".[cli]"
```

### `raygeo trace <path>`

Run adaptive clearing with per-step tracing and write a binary trace file:

```bash
raygeo trace /tmp/trace.bin
raygeo trace /tmp/trace.bin --scenario centre-island
raygeo trace /tmp/trace.bin --svg myshape.svg --tool-radius 2.5
```

Optional flags: `--scenario`, `--svg`, `--tool-radius`, `--advance`,
`--step-over`, `--step-length`, `--max-deflection-deg`, `--wall-margin`,
`--cut-z`, `--safe-z`, `--area-tolerance`.

### `raygeo print <path>`

Dump all trace records as a grep-friendly event log. Useful for quickly
scanning strategy outcomes (`strat=` and `rout=` columns).

```bash
raygeo print /tmp/trace.bin
```

### `raygeo inspect <path> [step]`

Open an interactive matplotlib viewer for a trace file. Step through
individual adaptive clearing iterations with keyboard or button controls:

- **Left/Right** — previous/next step
- **Shift+Left/Right** — previous/next segment
- **Home/End** — first/last step
- **M** — toggle MAT overlay

```bash
raygeo inspect /tmp/trace.bin
raygeo inspect /tmp/trace.bin 500   # start at step 500
```

### `raygeo profile`

Profile `adaptive_clearing` wall-clock performance for built-in or SVG
scenarios:

```bash
raygeo profile
raygeo profile --scenario island-routing
raygeo profile --svg myshape.svg --tool-radius 3.0
```

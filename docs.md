# Docs Generation Plan

## Motivation

Move API docs + visual example generation into the raygeo repository so that
`make docs` in raygeo produces everything needed for the website. This means:

- The rayforge repository no longer runs `scripts/update_api_docs.py` against
  the raygeo subdirectory — instead it simply copies `docs/` from raygeo.
- `tools/visual_test.py` is refactored so that its visual example generation
  code can be reused by the docs generation script (headless, non-Streamlit).
- Each "page" in the visual test becomes a doc page with an inline image.

## Plan

### 1. Refactor `tools/visual_test.py` into modular pieces

**Goal**: Extract plotting/rendering helpers into a reusable module, and keep
only the Streamlit UI glue in `visual_test.py`.

New file layout under `tools/`:

```
tools/
├── __init__.py
├── visual_test.py         # Streamlit app (~300-400 lines, import helpers)
├── cli.py                 # Argparse CLI ("raygeo-docs" entry point)
├── plot.py                # Pure plotting helpers (extracted from visual_test)
│   - _plot_geometry()
│   - _plot_ops()
│   - _arc_angles()
│   - _auto_limits()
│   - _geometry_to_mpath()
│   - _rasterize_geometries_to_mask()
│   - _make_pattern()
│   - _fill_rounded_rect()
│   - _plot_polygon()
├── examples/
│   ├── __init__.py
│   ├── geometry.py        # page_geometry → generate_geometry_example(save_to)
│   ├── polygon_boolean.py # page_polygon_boolean → generate_polygon_boolean_example(...)
│   ├── polygon_offset.py  # page_offset → ...
│   ├── image.py           # page_image → ...
│   ├── svg.py             # page_svg → ...
│   ├── tab_ops.py         # page_tabs → ...
│   ├── merge_lines.py     # page_merge_lines → ...
│   ├── overscan.py        # page_overscan → ...
│   ├── lead_in_out.py     # page_lead_in_out → ...
│   ├── rasterization.py   # page_rasterization → ...
│   ├── concave_hull.py    # page_concave_hull → ...
│   └── nesting.py         # page_nesting → ...
```

Each `examples/*.py` module exports one or more functions like:

```python
def generate_geometry_example(output_dir: Path) -> list[dict]:
    """Generate example images, return metadata for markdown embedding."""
```

Each function:
- Uses hardcoded/deterministic parameters (no streamlit widgets).
- Calls `plot.py` helpers to create `matplotlib.figure.Figure` objects.
- Saves PNGs to `output_dir`.
- Returns metadata: `{"title": ..., "description": ..., "images": [{"path": "relative/path.png", "caption": ...}]}`

### 2. Create CLI (`tools/cli.py`)

```bash
raygeo-docs generate [--output docs/] [--stubs python/raygeo]
```

Subcommands:
- `generate api` — generate API reference markdown from .pyi stubs (port of `stubs_to_markdown.py`)
- `generate examples` — generate visual example PNGs + markdown pages
- `generate all` — run both

The CLI is installed as a console_script via `pyproject.toml`.

### 3. Port `stubs_to_markdown.py` into `tools/cli.py`

The `stubs_to_markdown.py` from rayforge is copied into `tools/` (or inlined into
`tools/cli.py` as a `generate_api_docs()` function). It's ~980 lines of pure
Python with no rayforge-specific dependencies — a clean copy. We can put it at
`tools/api_docs.py`.

### 4. Define `docs/` output structure

```
docs/
├── index.md                       # Overview page (auto-generated or static)
├── api/                           # API reference (from stubs)
│   ├── raygeo.md
│   ├── raygeo.geo.md
│   ├── raygeo.geo.shape.polygon.md
│   └── ...
└── examples/                      # Visual examples (generated)
    ├── index.md                   # Gallery
    ├── geometry.md                # One page per example category
    ├── polygon-boolean.md
    ├── polygon-offset.md
    ├── image-processing.md
    ├── svg-parsing.md
    ├── tab-operations.md
    ├── merge-lines.md
    ├── overscan.md
    ├── lead-in-out.md
    ├── rasterization.md
    ├── concave-hull.md
    ├── nesting.md
    └── images/                    # PNG assets
        ├── geometry-rectangle.png
        ├── polygon-boolean-union.png
        └── ...
```

Each example markdown page:
- Has YAML frontmatter with a `sidebar_label`.
- Embeds the generated PNGs as `![caption](images/foo.png)`.
- Includes the metadata description text.

### 5. Update `Makefile`

Add:

```makefile
docs:
	python -m tools.cli generate all
```

And update the existing `visual` target (keep it working).

### 6. Update `pyproject.toml`

Add a `docs` optional dependency group:

```toml
[project.optional-dependencies]
docs = [
    "matplotlib",
    "numpy",
]
```

The `generate api` subcommand needs no extra deps (uses stdlib `ast`).

### 7. Rayforge integration

In `rayforge/scripts/update_api_docs.py`, replace the existing logic with:

```python
"""Copy pre-generated docs from raygeo into the website."""
import shutil
from pathlib import Path

RAYGEO_DOCS = Path("external/raygeo/docs")
WEBSITE_DOCS = Path("website/docs/developer/raygeo-api")

def main():
    if RAYGEO_DOCS.exists():
        shutil.copytree(RAYGEO_DOCS / "api", WEBSITE_DOCS, dirs_exist_ok=True)
    # Also copy examples to an appropriate location or reference them

if __name__ == "__main__":
    main()
```

Rayforge's build process runs `make -C external/raygeo docs` before building
the website, ensuring docs are always fresh.

### 8. Implementation order

1. Create `tools/plot.py` — extract all non-Streamlit plotting helpers from `visual_test.py`.
2. Create `tools/api_docs.py` — copy `stubs_to_markdown.py` from rayforge into raygeo.
3. Refactor `tools/visual_test.py` to import from `tools/plot.py` and `tools/examples/*`.
4. Create `tools/examples/` modules — one per page, deterministic, headless.
5. Create `tools/cli.py` with `generate api` and `generate examples` subcommands.
6. Update `Makefile` with `docs` target.
7. Update `pyproject.toml` with docs extras + console_scripts entry point.
8. Update `docs/README.md` or `docs/index.md` as a landing page.
9. Update rayforge's `update_api_docs.py` to just copy from raygeo's `docs/`.
10. Test: `make docs` produces `docs/` with api/ + examples/ + images/.

### 9. Non-goals / out of scope

- Not converting the Streamlit app to a different framework — just extracting
  shared code so the CLI can run headlessly.
- Not changing the markdown format for the website — keeping Docusaurus-compatible
  frontmatter so rayforge's current sidebar config still works.
- Not generating API docs for Rust-only types (only Python stubs are documented).

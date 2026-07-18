"""Generate golden fixtures for the Rust transformer dispatch.

For each fixture case this writes three files into
``tests/transform/fixtures/``:

- ``<name>.ops.json``   -- input Ops (``Ops.to_dict()``)
- ``<name>.spec.json``  -- the typed Rust spec (``__dict__`` of the
  pyclass, which exposes its ``#[pyo3(get)]`` fields)
- ``<name>.expected_ops.json`` -- output Ops after
  ``Ops.apply_transformers([spec])``
"""

from __future__ import annotations

import json
from pathlib import Path

from raygeo.geo import Geometry
from raygeo.ops import Ops
from raygeo.ops.transform.bidir_scan_offset import BidirScanOffsetSpec
from raygeo.ops.transform.clip import CropSpec
from raygeo.ops.transform.lead_in_out import LeadInOutSpec
from raygeo.ops.transform.merge_lines import MergeLinesSpec
from raygeo.ops.transform.multipass import MultiPassSpec
from raygeo.ops.transform.optimize import OptimizeSpec
from raygeo.ops.transform.overscan import OverscanSpec
from raygeo.ops.transform.smooth import SmoothSpec
from raygeo.ops.transform.tabs import TabsSpec

FIXTURE_DIR = Path(__file__).parent


def _spec_to_dict(spec) -> dict:
    """Extract the public ``#[pyo3(get)]`` fields from a spec pyclass."""
    return {
        name: getattr(spec, name)
        for name in dir(spec)
        if not name.startswith("_") and not callable(getattr(spec, name))
    }


def _write(name: str, ops: Ops, spec) -> None:
    ops_path = FIXTURE_DIR / f"{name}.ops.json"
    spec_path = FIXTURE_DIR / f"{name}.spec.json"
    expected_path = FIXTURE_DIR / f"{name}.expected_ops.json"

    ops_path.write_text(json.dumps(ops.to_dict(), indent=2), encoding="utf-8")
    spec_path.write_text(
        json.dumps(_spec_to_dict(spec), indent=2), encoding="utf-8"
    )

    out = ops.copy()
    out.apply_transformers([spec])
    expected_path.write_text(
        json.dumps(out.to_dict(), indent=2), encoding="utf-8"
    )


def _square_ops() -> Ops:
    ops = Ops()
    ops.move_to(0, 0, 0)
    ops.line_to(10, 0, 0)
    ops.line_to(10, 10, 0)
    ops.line_to(0, 10, 0)
    ops.line_to(0, 0, 0)
    return ops


def _raster_ops() -> Ops:
    ops = Ops()
    for y in (0.0, 2.0, 4.0):
        ops.move_to(0, y, 0)
        ops.scan_to(10, y, 0)
    return ops


def _stock_geometry() -> Geometry:
    geo = Geometry()
    geo.move_to(1, 1, 0)
    geo.line_to(9, 1, 0)
    geo.line_to(9, 9, 0)
    geo.line_to(1, 9, 0)
    geo.close_path()
    return geo


def main() -> None:
    _write(
        "smooth_basic",
        _square_ops(),
        SmoothSpec(amount=20, corner_angle_threshold=45),
    )
    _write(
        "optimize_basic",
        _square_ops(),
        OptimizeSpec(
            allow_flip=True,
            preserve_first=False,
            preserve_order=[],
        ),
    )
    _write("merge_lines_basic", _square_ops(), MergeLinesSpec(tolerance=0.01))
    _write("overscan_basic", _raster_ops(), OverscanSpec(distance_mm=1.0))
    _write(
        "lead_in_out_basic",
        _square_ops(),
        LeadInOutSpec(lead_in_mm=1.0, lead_out_mm=1.0),
    )
    _write(
        "multipass_basic",
        _square_ops(),
        MultiPassSpec(passes=3, z_step_down=0.0),
    )
    _write(
        "multipass_zstep",
        _square_ops(),
        MultiPassSpec(passes=2, z_step_down=0.5),
    )
    _write(
        "bidir_scan_offset_basic",
        _raster_ops(),
        BidirScanOffsetSpec(offset_mm=0.3),
    )
    polygons = _stock_geometry().to_polygons(0.01)
    _write(
        "crop_basic",
        _square_ops(),
        CropSpec(tolerance=0.01, offset=0.0, regions=polygons),
    )
    _write(
        "tabs_gap_basic",
        _square_ops(),
        TabsSpec(tab_power=0.0, original_power=1.0, clips=[(5.0, 0.0, 2.0)]),
    )
    _write(
        "tabs_power_basic",
        _square_ops(),
        TabsSpec(tab_power=0.5, original_power=1.0, clips=[(5.0, 0.0, 2.0)]),
    )


if __name__ == "__main__":
    main()
    print(f"Wrote fixtures to {FIXTURE_DIR}")

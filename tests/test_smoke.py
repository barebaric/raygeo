"""Smoke test for the merged native extension.

Runs on every platform in CI. It proves that the geometry/ops core, the
machine-code converters and the driver namespace are all present and that
an Ops sequence can be encoded through the same extension.
"""

import json

import pytest

import raygeo.driver
from raygeo.ops import Ops
from raygeo.ops.convert import Encoder, GcodeDialectSpec, GcodeSpec

_CONTEXT = {
    "gcode_precision": 3,
    "max_travel_speed": 6000.0,
    "default_head_uid": "h0",
    "heads": [{"uid": "h0", "tool_number": 0, "max_power": 100.0}],
    "active_wcs": "",
    "layer_wcs": {},
    "macros": {},
    "path_vars": {},
    "layer_path_vars": {},
    "workpiece_path_vars": {},
}


@pytest.mark.smoke
def test_merged_extension_smoke():
    ops = Ops()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)

    out = ops.encode(
        Encoder(
            GcodeSpec(
                dialect=GcodeDialectSpec(),
                context_json=json.dumps(_CONTEXT),
            )
        )
    )
    assert out.text
    assert "G1" in out.text

    assert raygeo.driver.Driver is not None
    assert raygeo.driver.grbl.GrblSerialDriver is not None

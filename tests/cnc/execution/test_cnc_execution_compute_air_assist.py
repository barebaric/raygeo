"""Air assist injection through the pipeline Compute stage.

Verifies that a `ComputePayload` with `air_assist` set injects the
matching `SetAirAssist` command into the assembled Ops, and that
G-code encoding turns that command into M8/M9.

This covers the regression where step air assist settings never
reached the ops stream after the encoder pipeline moved into raygeo.
"""

from typing import Optional

from conftest import (
    collect_completions,
    make_square_part,
    result_ops,
)

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.ops import Ops
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.frame import FrameSpec
from raygeo.ops.convert import GcodeDialectSpec
from raygeo.ops.state import AirAssistMode
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _node(key: str, air_assist: Optional[AirAssistMode] = None) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=make_square_part(),
            params=ComputePayload(
                assembler=Assembler(FrameSpec()),
                air_assist=air_assist,
            ),
        ),
    )


def _run(key: str, air_assist: Optional[AirAssistMode]) -> Ops:
    completed, _ = collect_completions([_node(key, air_assist)])
    assert len(completed) == 1
    return result_ops(completed[0])


def _air_assist_cmds(ops: Ops) -> list[str]:
    return [
        c["air_assist"]
        for c in ops.to_dict()["commands"]
        if c.get("type") == "SET_AIR_ASSIST"
    ]


def test_default_no_air_assist_command():
    ops = _run("n1", None)
    assert _air_assist_cmds(ops) == []


def test_air_assist_on_injects_set_air_assist():
    ops = _run("n2", AirAssistMode.ON)
    assert _air_assist_cmds(ops) == ["On"]


def test_air_assist_off_injects_set_air_assist():
    ops = _run("n3", AirAssistMode.OFF)
    assert _air_assist_cmds(ops) == ["Off"]


def test_air_assist_property_roundtrip():
    payload = ComputePayload(assembler=Assembler(FrameSpec()))
    assert payload.air_assist is None
    payload.air_assist = AirAssistMode.ON
    assert payload.air_assist == AirAssistMode.ON
    payload.air_assist = AirAssistMode.OFF
    assert payload.air_assist == AirAssistMode.OFF
    payload.air_assist = None
    assert payload.air_assist is None


def test_air_assist_on_encodes_m8():
    ops = _run("n4", AirAssistMode.ON)
    result = ops.to_gcode(GcodeDialectSpec(), {})
    assert "M8" in result["text"]


def test_air_assist_off_encodes_no_redundant_command():
    """The encoder starts in the off state, so an initial OFF must not
    produce a redundant M9."""
    ops = _run("n5", AirAssistMode.OFF)
    result = ops.to_gcode(GcodeDialectSpec(), {})
    assert "M8" not in result["text"]
    assert "M9" not in result["text"]


def test_default_encodes_no_air_assist():
    ops = _run("n6", None)
    result = ops.to_gcode(GcodeDialectSpec(), {})
    assert "M8" not in result["text"]
    assert "M9" not in result["text"]

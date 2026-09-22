"""Exit-criteria tests for Slice A3: Frame assembler dispatched
through the pipeline Compute stage.

Verifies that:
- The pipeline Compute stage dispatches a `FrameSpec` through
  `Box<dyn Assembler>` and produces a `ComputeResult`.
- The produced `Ops` is byte-identical to what the standalone
  `frame()` pyfunction produces for the same inputs.
- `is_scalable` is `True` for frame (vector output).
- Different `cut_side` parameters produce different output.
"""

import math
from typing import Optional

import pytest
from conftest import (
    collect_completions,
    compute_result,
    make_square_part,
    result_ops,
)

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.frame import FrameSpec, frame
from raygeo.ops.part import Part
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _frame_node(
    key: str,
    part: Optional[Part] = None,
    spec: Optional[FrameSpec] = None,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=part or make_square_part(),
            params=ComputePayload(assembler=Assembler(spec or FrameSpec())),
        ),
    )


def _run_one(node):
    completed, _ = collect_completions([node])
    assert len(completed) == 1
    return completed[0]


def test_frame_compute_succeeds():
    c = _run_one(_frame_node("f1"))
    assert c.error is None
    assert c.output is not None


def test_frame_compute_is_scalable_true():
    c = _run_one(_frame_node("f1"))
    out = compute_result(c)
    assert out.is_scalable is True


def test_frame_pipeline_matches_direct_call():
    part = Part(size_mm=(10.0, 10.0))
    pipe_part = Part(size_mm=(10.0, 10.0))
    c = _run_one(
        _frame_node(
            "match",
            part=pipe_part,
            spec=FrameSpec(offset_mm=1.0, cut_side="outside"),
        )
    )
    direct = frame(part, offset_mm=1.0, cut_side="outside")
    pipe_ops = result_ops(c).to_dict()
    direct_ops = direct.ops.to_dict()
    assert pipe_ops["commands"][0] == {"type": "SET_POWER", "power": 0.0}
    assert pipe_ops["commands"][1:] == direct_ops["commands"]
    assert pipe_ops["last_move_to"] == direct_ops["last_move_to"]


def test_frame_cut_side_changes_output():
    center = result_ops(
        _run_one(
            _frame_node(
                "c",
                spec=FrameSpec(offset_mm=1.5, cut_side="centerline"),
            )
        )
    ).to_dict()
    outside = result_ops(
        _run_one(
            _frame_node(
                "o",
                spec=FrameSpec(offset_mm=1.5, cut_side="outside"),
            )
        )
    ).to_dict()
    assert center != outside


def _points(ops) -> list[tuple[float, float]]:
    points: list[tuple[float, float]] = []
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] == "MOVE_TO":
            points.append((cmd["end"][0], cmd["end"][1]))
        elif cmd["type"] == "LINE_TO":
            points.append((cmd["end"][0], cmd["end"][1]))
    return points


def test_frame_corner_radius_rounds_corners():
    c = _run_one(_frame_node("rounded", spec=FrameSpec(corner_radius=3.0)))
    points = _points(result_ops(c))
    assert (10.0, 10.0) not in points
    corner_arc = [(x, y) for x, y in points if x > 6.0 and y > 6.0]
    assert corner_arc
    for x, y in corner_arc:
        assert math.hypot(x - 7.0, y - 7.0) == pytest.approx(3.0, abs=1e-6)


def test_frame_corner_radius_zero_keeps_sharp_corners():
    c = _run_one(_frame_node("sharp", spec=FrameSpec(corner_radius=0.0)))
    points = _points(result_ops(c))
    assert (10.0, 0.0) in points
    assert (10.0, 10.0) in points


def test_frame_corner_radius_clamps_to_frame():
    c = _run_one(_frame_node("clamped", spec=FrameSpec(corner_radius=50.0)))
    assert c.error is None
    points = _points(result_ops(c))
    xs = [x for x, _ in points]
    ys = [y for _, y in points]
    assert min(xs) == pytest.approx(0.0)
    assert max(xs) == pytest.approx(10.0)
    assert min(ys) == pytest.approx(0.0)
    assert max(ys) == pytest.approx(10.0)

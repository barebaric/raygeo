"""Exit-criteria tests for Slice A3: Shrinkwrap assembler dispatched
through the pipeline Compute stage.

Verifies that:
- The pipeline Compute stage dispatches a `ShrinkwrapSpec` through
  `Box<dyn Assembler>` and produces a `ComputeResult`.
- The produced `Ops` is byte-identical to what the standalone
  `shrinkwrap()` pyfunction produces for the same inputs.
- `is_scalable` is `False` for shrinkwrap (hull is image-derived).
- Different `gravity` parameters produce different output.
"""

from typing import Optional

import numpy as np
from conftest import (
    collect_completions,
    compute_result,
    result_ops,
)

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.shrinkwrap import ShrinkwrapSpec, shrinkwrap
from raygeo.ops.part import Part
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _filled_part(fill: int = 255, size_mm=(10.0, 10.0), ppm=(10.0, 10.0)):
    part = Part(size_mm=size_mm, pixels_per_mm=ppm)
    w = int(size_mm[0] * ppm[0])
    h = int(size_mm[1] * ppm[1])
    part.image = np.full((h, w), fill, dtype=np.uint8)
    return part


def _shrinkwrap_node(
    key: str,
    part: Optional[Part] = None,
    spec: Optional[ShrinkwrapSpec] = None,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=part or _filled_part(),
            params=ComputePayload(
                assembler=Assembler(
                    spec or ShrinkwrapSpec(gravity=0.0, cut_side="outer")
                )
            ),
        ),
    )


def _run_one(node):
    completed, _ = collect_completions([node])
    assert len(completed) == 1
    return completed[0]


def test_shrinkwrap_compute_succeeds():
    c = _run_one(_shrinkwrap_node("s1"))
    assert c.error is None
    assert c.output is not None
    out = compute_result(c)
    assert len(out.ops) > 0


def test_shrinkwrap_compute_is_scalable_false():
    c = _run_one(_shrinkwrap_node("s1"))
    out = compute_result(c)
    assert out.is_scalable is False


def test_shrinkwrap_pipeline_matches_direct_call():
    spec = ShrinkwrapSpec(
        gravity=0.0,
        kerf_mm=0.0,
        path_offset_mm=0.0,
        cut_side="outer",
        arc_tolerance=0.0,
        allow_arcs=False,
        supports_curves=False,
    )
    pipe_part = _filled_part()
    direct_part = _filled_part()
    c = _run_one(_shrinkwrap_node("match", part=pipe_part, spec=spec))
    direct = shrinkwrap(
        direct_part,
        gravity=0.0,
        kerf_mm=0.0,
        path_offset_mm=0.0,
        cut_side="outer",
        arc_tolerance=0.0,
        allow_arcs=False,
        supports_curves=False,
    )
    pipe_ops = result_ops(c).to_dict()
    direct_ops = direct.ops.to_dict()
    assert pipe_ops["commands"][0] == {"type": "SET_POWER", "power": 0.0}
    assert pipe_ops["commands"][1:] == direct_ops["commands"]
    assert pipe_ops["last_move_to"] == direct_ops["last_move_to"]


def test_shrinkwrap_gravity_changes_output():
    low = result_ops(
        _run_one(
            _shrinkwrap_node(
                "low", spec=ShrinkwrapSpec(gravity=0.0, cut_side="outer")
            )
        )
    ).to_dict()
    high = result_ops(
        _run_one(
            _shrinkwrap_node(
                "high", spec=ShrinkwrapSpec(gravity=0.5, cut_side="outer")
            )
        )
    ).to_dict()
    assert low != high

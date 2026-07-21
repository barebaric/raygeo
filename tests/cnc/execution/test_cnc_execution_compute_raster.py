"""Exit-criteria tests for Slice A3: Raster assembler dispatched
through the pipeline Compute stage.

Verifies that:
- The pipeline Compute stage dispatches a `RasterSpec` through
  `Box<dyn Assembler>` and produces a `ComputeResult`.
- The produced `Ops` is byte-identical to what the standalone
  `raster()` pyfunction produces for the same inputs.
- `is_scalable` is `False` for raster (scanline spacing is physical).
- Different `mode` parameters produce different output.
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
from raygeo.ops.assembly.raster import RasterSpec, raster
from raygeo.ops.part import Part
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _filled_part(fill: int = 255, size_mm=(10.0, 10.0), ppm=(10.0, 10.0)):
    part = Part(size_mm=size_mm, pixels_per_mm=ppm)
    w = int(size_mm[0] * ppm[0])
    h = int(size_mm[1] * ppm[1])
    part.image = np.full((h, w), fill, dtype=np.uint8)
    return part


def _raster_node(
    key: str,
    part: Optional[Part] = None,
    spec: Optional[RasterSpec] = None,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=part or _filled_part(),
            params=ComputePayload(
                assembler=Assembler(spec or RasterSpec(mode="mask_scan"))
            ),
        ),
    )


def _run_one(node):
    completed, _ = collect_completions([node])
    assert len(completed) == 1
    return completed[0]


def test_raster_compute_succeeds():
    c = _run_one(_raster_node("r1"))
    assert c.error is None
    assert c.output is not None
    out = compute_result(c)
    assert len(out.ops) > 0


def test_raster_compute_is_scalable_false():
    c = _run_one(_raster_node("r1"))
    out = compute_result(c)
    assert out.is_scalable is False


def test_raster_pipeline_matches_direct_call():
    spec = RasterSpec(mode="mask_scan", line_interval_mm=1.0, step_power=0.2)
    pipe_part = _filled_part()
    direct_part = _filled_part()
    c = _run_one(_raster_node("match", part=pipe_part, spec=spec))
    direct = raster(
        direct_part, mode="mask_scan", line_interval_mm=1.0, step_power=0.2
    )
    assert result_ops(c).to_dict() == direct.ops.to_dict()


def test_raster_pipeline_matches_power_modulated():
    fill = 128
    alpha = np.full((100, 100), 200, dtype=np.uint8)
    spec = RasterSpec(
        mode="power_modulated",
        line_interval_mm=1.0,
        sample_interval_mm=0.1,
        step_power=0.1,
        alpha=alpha.flatten().tolist(),
    )
    pipe_part = _filled_part(fill=fill)
    direct_part = _filled_part(fill=fill)
    c = _run_one(_raster_node("pm", part=pipe_part, spec=spec))
    direct = raster(
        direct_part,
        alpha=alpha,
        mode="power_modulated",
        line_interval_mm=1.0,
        sample_interval_mm=0.1,
        step_power=0.1,
    )
    assert result_ops(c).to_dict() == direct.ops.to_dict()


def test_raster_mode_changes_output():
    mask = result_ops(
        _run_one(
            _raster_node(
                "mask",
                spec=RasterSpec(mode="mask_scan", line_interval_mm=1.0),
            )
        )
    ).to_dict()
    multi = result_ops(
        _run_one(
            _raster_node(
                "multi",
                spec=RasterSpec(
                    mode="multi_pass",
                    line_interval_mm=1.0,
                    num_depth_levels=3,
                ),
            )
        )
    ).to_dict()
    assert mask != multi

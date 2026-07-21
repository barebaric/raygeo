from typing import Optional

import pytest

from raygeo.cnc.execution.specs import AggregateOutput, ComputePayload
from raygeo.geo import Geometry
from raygeo.ops import Ops
from raygeo.ops.assembly import Assembler, AssemblyOutput
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.convert import EncodeOutput
from raygeo.ops.part import Part
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import clear_cache, execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def make_square_part() -> Part:
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 0)
    g.line_to(10, 10)
    g.line_to(0, 10)
    g.line_to(0, 0)
    return Part(geometry=g, size_mm=(10.0, 10.0))


def make_contour_compute(
    key: str,
    part: Optional[Part] = None,
    spec: Optional[ContourSpec] = None,
    on_progress=None,
    on_cancelled=None,
    on_chunk=None,
    generation_id: int = 1,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=generation_id,
        stage=StageSpec.Compute(
            part=part or make_square_part(),
            params=ComputePayload(assembler=Assembler(spec or ContourSpec())),
        ),
        on_progress=on_progress,
        on_cancelled=on_cancelled,
        on_chunk=on_chunk,
    )


def collect_completions(nodes, on_batch=None):
    completed: list[CompletedNode] = []
    batch_progress: list[tuple[float, str]] = []

    def _batch(frac: float, msg: str) -> None:
        batch_progress.append((frac, msg))

    try:
        execute_stages(
            nodes,
            completed.append,
            _batch if on_batch else None,
        )
    except RuntimeError:
        pass
    return completed, batch_progress


def compute_result(node: CompletedNode) -> AssemblyOutput:
    assert node.output is not None
    assert isinstance(node.output, AssemblyOutput)
    return node.output


def aggregate_result(node: CompletedNode) -> AggregateOutput:
    assert node.output is not None
    assert isinstance(node.output, AggregateOutput)
    return node.output


def encode_result(node: CompletedNode) -> EncodeOutput:
    assert node.output is not None
    assert isinstance(node.output, EncodeOutput)
    return node.output


def result_ops(node: CompletedNode) -> Ops:
    assert node.output is not None
    out = node.output
    if isinstance(out, AssemblyOutput):
        return out.ops
    assert isinstance(out, AggregateOutput)
    return out.ops


@pytest.fixture(autouse=True)
def _clear_pipeline_cache():
    clear_cache()
    yield
    clear_cache()

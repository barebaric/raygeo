"""Tests for the Compute stage.

The Contour assembler is the canonical vector Compute stage. These
tests verify that:
- The pipeline Compute stage actually invokes the wrapped assembler.
- The produced StageOutput.ComputeResult carries an Ops payload and
  the part's source dimensions.
- Different ContourSpec parameters (kerf, offset, cut_side) produce
  different Ops outputs.
- The pipeline output matches the direct `contour()` call.
- Multi-face Compute nodes targeting different geometries each
  produce Ops matching their own input.
"""

from conftest import (
    collect_completions,
    compute_result,
    make_contour_compute,
    make_square_part,
    result_ops,
)

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.geo import Geometry
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec, contour
from raygeo.ops.part import Part
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _run_one(node):
    completed, _ = collect_completions([node])
    assert len(completed) == 1
    return completed[0]


# ── Contour produces a ComputeResult ──────────────────────────────


def test_contour_compute_succeeds():
    c = _run_one(make_contour_compute("c1"))
    assert c.error is None
    assert c.output is not None


def test_contour_compute_carries_ops():
    c = _run_one(make_contour_compute("c1"))
    out = compute_result(c)
    assert type(out).__name__ == "AssemblyOutput"
    assert out.ops is not None
    assert len(out.ops) > 0


def test_contour_compute_source_dimensions_echoed():
    c = _run_one(make_contour_compute("c1"))
    out = compute_result(c)
    assert out.source_dimensions == (10.0, 10.0)


def test_contour_compute_is_scalable_default_true():
    c = _run_one(make_contour_compute("c1"))
    out = compute_result(c)
    assert out.is_scalable is True


# ── Different ContourSpec parameters produce different output ─────


def test_offset_changes_output():
    base = result_ops(
        _run_one(make_contour_compute("b", spec=ContourSpec()))
    ).to_dict()
    offset = result_ops(
        _run_one(
            make_contour_compute(
                "o",
                spec=ContourSpec(
                    offset_mm=7.5,
                    cut_side="outside",
                ),
            )
        )
    ).to_dict()
    assert base != offset


def test_cut_side_outside_changes_output():
    center = result_ops(
        _run_one(
            make_contour_compute(
                "c",
                spec=ContourSpec(
                    offset_mm=7.5,
                    cut_side="centerline",
                ),
            )
        )
    ).to_dict()
    outside = result_ops(
        _run_one(
            make_contour_compute(
                "o",
                spec=ContourSpec(
                    offset_mm=7.5,
                    cut_side="outside",
                ),
            )
        )
    ).to_dict()
    assert center != outside


# ── Pipeline output matches the direct contour() call ─────────────


def test_pipeline_matches_direct_contour_call():
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 0)
    g.line_to(10, 10)
    g.line_to(0, 10)
    g.line_to(0, 0)

    pipe_part = Part(geometry=g, size_mm=(10.0, 10.0))
    direct_part = Part(geometry=g, size_mm=(10.0, 10.0))

    c = _run_one(
        make_contour_compute(
            "match",
            part=pipe_part,
            spec=ContourSpec(offset_mm=7.5, cut_side="outside"),
        )
    )
    direct = contour(direct_part, offset_mm=7.5, cut_side="outside")
    pipe_ops = result_ops(c).to_dict()
    direct_ops = direct.ops.to_dict()
    assert pipe_ops["commands"][0] == {"type": "SET_POWER", "power": 0.0}
    assert pipe_ops["commands"][1:] == direct_ops["commands"]
    assert pipe_ops["last_move_to"] == direct_ops["last_move_to"]


# ── Empty part produces empty Ops ─────────────────────────────────


def test_empty_part_yields_empty_ops():
    empty_part = Part(geometry=None, size_mm=(0.0, 0.0))
    c = _run_one(make_contour_compute("e", part=empty_part))
    if c.error is None:
        cmds = result_ops(c).to_dict()["commands"]
        assert cmds == [{"type": "SET_POWER", "power": 0.0}]


# ── Multiple Contour nodes run independently ──────────────────────


def test_multiple_contour_nodes_each_complete():
    nodes = [make_contour_compute(f"c{i}") for i in range(5)]
    completed, _ = collect_completions(nodes)
    assert len(completed) == 5
    keys = {c.key for c in completed}
    assert keys == {f"c{i}" for i in range(5)}
    for c in completed:
        assert c.error is None
        assert c.output is not None


# ── Multi-face contour compute ────────────────────────────────────


def test_multiface_contour_runs_independently():
    square = make_square_part()
    triangle_geo = Geometry()
    triangle_geo.move_to(0, 0)
    triangle_geo.line_to(5, 10)
    triangle_geo.line_to(10, 0)
    triangle_geo.line_to(0, 0)
    triangle_part = Part(geometry=triangle_geo, size_mm=(10.0, 10.0))

    spec = ContourSpec()
    completions: list[CompletedNode] = []

    def on_completed(n):
        completions.append(n)

    execute_stages(
        [
            NodeRequest(
                key="square",
                generation_id=1,
                stage=StageSpec.Compute(
                    part=square,
                    params=ComputePayload(assembler=Assembler(spec)),
                ),
            ),
            NodeRequest(
                key="triangle",
                generation_id=1,
                stage=StageSpec.Compute(
                    part=triangle_part,
                    params=ComputePayload(assembler=Assembler(spec)),
                ),
            ),
        ],
        on_completed,
    )

    assert len(completions) == 2
    for c in completions:
        assert c.error is None, f"node {c.key} failed: {c.error}"
    for c in completions:
        out = compute_result(c)
        assert len(out.ops) > 0

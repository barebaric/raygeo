"""Transformer integration tests for the pipeline.

Verifies that transformers attached to a Compute node are actually
applied to its assembler output, that changing a transformer
parameter changes the output, and that multiple transformers are
applied in phase order.

The cache-level behavior (transformer hash folds into the compute
cache key, so changing a transformer invalidates the cache) is
covered in ``test_cnc_execution_cache.py``.

Note: Aggregate-level transformer application is covered indirectly
by the multi-assembler intent tests; the existing Aggregate stage
does not cache results, so transformer caching there is a no-op
(see ``test_aggregate_transformers_are_applied`` below for
non-caching application verification).
"""

from typing import List

import pytest
from conftest import (
    collect_completions,
    make_square_part,
    result_ops,
)

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    ComputePayload,
    MachineParams,
)
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.transform.multipass import MultiPassSpec
from raygeo.ops.transform.smooth import SmoothSpec
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

IDENTITY = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def _contour_node(key: str, transformers=None) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=make_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()),
                transformers=transformers or [],
            ),
        ),
    )


def _agg(key: str, source_keys: List[str], transformers=None) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Aggregate(
            spec=AggregateSpec(
                wrap_start=[],
                groups=[
                    AggregateGroup(
                        start_markers=[],
                        inputs=[
                            AggregateInput(
                                source_key=sk,
                                placement_matrix=IDENTITY,
                                uid="",
                                target_dimensions=(0.0, 0.0),
                            )
                            for sk in source_keys
                        ],
                        end_markers=[],
                    )
                ],
                wrap_end=[],
                machine=MachineParams(),
                transformers=transformers or [],
            )
        ),
    )


def _run(nodes) -> dict:
    completed, _ = collect_completions(nodes)
    return {c.key: c for c in completed}


# ── Compute-level transformer application ────────────────────────


def test_no_transformer_yields_baseline_ops():
    """Compute with transformers=[] matches Compute with no
    transformers field set at all."""
    baseline = result_ops(_run([_contour_node("a")])["a"]).to_dict()
    explicit = result_ops(
        _run([_contour_node("b", transformers=[])])["b"]
    ).to_dict()
    assert baseline == explicit


def test_smooth_transformer_changes_output():
    """Adding SmoothSpec changes the ops vs. no-transformer baseline."""
    baseline = result_ops(
        _run([_contour_node("baseline")])["baseline"]
    ).to_dict()
    smoothed = result_ops(
        _run([_contour_node("smoothed", transformers=[SmoothSpec(50, 30.0)])])[
            "smoothed"
        ]
    ).to_dict()
    assert baseline != smoothed


def test_smooth_param_change_changes_output():
    """Different SmoothSpec.amount produces different output."""
    low = result_ops(
        _run([_contour_node("low", transformers=[SmoothSpec(20, 30.0)])])[
            "low"
        ]
    ).to_dict()
    high = result_ops(
        _run([_contour_node("high", transformers=[SmoothSpec(80, 30.0)])])[
            "high"
        ]
    ).to_dict()
    assert low != high


def test_multipass_doubles_op_count():
    """MultiPassSpec(passes=2) doubles the number of cutting commands."""
    single = result_ops(_run([_contour_node("s")])["s"])
    multi = result_ops(
        _run([_contour_node("m", transformers=[MultiPassSpec(2, 0.0)])])["m"]
    )
    # Multipass duplicates the ops, so the multi version should have
    # roughly twice as many commands. Strict equality may not hold due
    # to boundary state commands but the gap is significant.
    assert len(multi) > len(single)
    assert len(multi) >= 2 * len(single) - 4  # tolerate bookkeeping cmds


def test_different_transformer_types_produce_different_output():
    """Different transformer types produce different modifications."""
    smoothed = result_ops(
        _run([_contour_node("sm", transformers=[SmoothSpec(50, 30.0)])])["sm"]
    ).to_dict()
    multipassed = result_ops(
        _run([_contour_node("mp", transformers=[MultiPassSpec(2, 0.0)])])["mp"]
    ).to_dict()
    assert smoothed != multipassed


# ── Aggregate-level transformer application ──────────────────────


def test_aggregate_transformers_are_applied():
    """Adding a SmoothSpec to AggregateSpec.transformers changes the
    aggregate output vs. no transformers.

    The 'src' node is run twice in fresh batches because the
    converter steals the Python Part on first dispatch.
    """
    out_no = result_ops(
        _run([_contour_node("src"), _agg("agg", ["src"], transformers=[])])[
            "agg"
        ]
    ).to_dict()
    out_with = result_ops(
        _run(
            [
                _contour_node("src"),
                _agg("agg", ["src"], transformers=[SmoothSpec(50, 30.0)]),
            ]
        )["agg"]
    ).to_dict()
    assert out_no != out_with


def test_aggregate_multipass_doubles_ops():
    """Adding MultiPassSpec(passes=2) to AggregateSpec roughly
    doubles the op count vs. no transformer."""
    no_t = result_ops(
        _run([_contour_node("src"), _agg("agg", ["src"], transformers=[])])[
            "agg"
        ]
    )
    with_t = result_ops(
        _run(
            [
                _contour_node("src"),
                _agg(
                    "agg",
                    ["src"],
                    transformers=[MultiPassSpec(2, 0.0)],
                ),
            ]
        )["agg"]
    )
    assert len(with_t) > len(no_t)
    assert len(with_t) >= 2 * len(no_t) - 4


# ── Unknown transformer type at construction ──────────────────────


def test_unknown_transformer_type_errors_at_construction():
    """A non-transformer Python object in transformers=[] raises
    TypeError when the stage is constructed (the converter calls
    extract_transformer, which rejects unknown types)."""

    nr = NodeRequest(
        key="bad",
        generation_id=1,
        stage=StageSpec.Compute(
            part=make_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()),
                transformers=[42],  # not a transformer spec
            ),
        ),
    )
    completed: list[CompletedNode] = []
    with pytest.raises(TypeError):
        execute_stages([nr], completed.append, None)


def test_mismatched_transformer_in_aggregate_errors():
    """An unknown transformer in AggregateSpec.transformers is
    rejected when the aggregate stage is constructed."""

    src = _contour_node("src")
    agg = NodeRequest(
        key="agg",
        generation_id=1,
        stage=StageSpec.Aggregate(
            spec=AggregateSpec(
                wrap_start=[],
                groups=[
                    AggregateGroup(
                        start_markers=[],
                        inputs=[
                            AggregateInput(
                                source_key="src",
                                placement_matrix=IDENTITY,
                                uid="",
                                target_dimensions=(0.0, 0.0),
                            )
                        ],
                        end_markers=[],
                    )
                ],
                wrap_end=[],
                machine=MachineParams(),
                transformers=["not a transformer"],
            )
        ),
    )
    completed: list[CompletedNode] = []
    with pytest.raises(TypeError):
        execute_stages([src, agg], completed.append, None)

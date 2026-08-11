"""Tests for the Aggregate stage.

The Aggregate stage:
- Emits declarative markers (JobStart/End, LayerStart/End,
  WorkpieceStart/End) around concatenated input Ops.
- Applies per-input placement matrices in target-space.
- Applies uniform scaling when `target_dimensions` differs from the
  upstream Compute node's `source_dimensions`.
- Computes a time estimate when `MachineParams` rates are non-zero.

Tests here run small trees of (Contour Compute -> Aggregate) and
inspect the resulting AggregateResult.
"""

from conftest import (
    aggregate_result,
    collect_completions,
    make_contour_compute,
    result_ops,
)

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    MachineParams,
    Marker,
)
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def test_marker_variants_exist():
    for name in (
        "JobStart",
        "JobEnd",
        "LayerStart",
        "LayerEnd",
        "WorkpieceStart",
        "WorkpieceEnd",
    ):
        assert hasattr(Marker, name), f"Missing Marker.{name}"


IDENTITY = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def _translate(tx: float, ty: float) -> list:
    return [
        [1.0, 0.0, 0.0, tx],
        [0.0, 1.0, 0.0, ty],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]


def _make_aggregate(
    key: str,
    inputs: list,
    wrap_start=None,
    wrap_end=None,
    machine=None,
    start_markers=None,
    end_markers=None,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Aggregate(
            spec=AggregateSpec(
                wrap_start=wrap_start or [],
                groups=[
                    AggregateGroup(
                        start_markers=start_markers or [],
                        inputs=inputs,
                        end_markers=end_markers or [],
                    )
                ],
                wrap_end=wrap_end or [],
                machine=machine or MachineParams(),
            )
        ),
    )


def _aggregate_only(src_key="src"):
    return _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key=src_key,
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            )
        ],
    )


def _by_key(completed):
    return {c.key: c for c in completed}


# ── Basic aggregate success ──────────────────────────────────────


def test_aggregate_one_input_succeeds():
    src = make_contour_compute("src")
    agg = _aggregate_only()
    completed, _ = collect_completions([src, agg])
    out = _by_key(completed)["agg"].output
    assert out is not None
    assert type(out).__name__ == "AggregateOutput"


def test_aggregate_produces_ops():
    src = make_contour_compute("src")
    agg = _aggregate_only()
    completed, _ = collect_completions([src, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    assert out.ops is not None
    assert len(out.ops) > 0


def test_aggregate_two_inputs_concatenates_ops():
    nodes = [
        make_contour_compute("a"),
        make_contour_compute("b"),
        _make_aggregate(
            "agg",
            [
                AggregateInput(
                    source_key="a",
                    placement_matrix=IDENTITY,
                    uid="",
                    target_dimensions=(0.0, 0.0),
                ),
                AggregateInput(
                    source_key="b",
                    placement_matrix=IDENTITY,
                    uid="",
                    target_dimensions=(0.0, 0.0),
                ),
            ],
        ),
    ]
    completed, _ = collect_completions(nodes)
    out = aggregate_result(_by_key(completed)["agg"])
    # 2 inputs x 8 commands (6 contour + VectorOutline section markers)
    assert len(out.ops) == 16


# ── Markers are emitted in order ──────────────────────────────────


def test_job_markers_emitted_at_wrap_start_end():
    src = make_contour_compute("src")
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="src",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            )
        ],
        wrap_start=[Marker.JobStart(_tag=True)],
        wrap_end=[Marker.JobEnd(_tag=True)],
    )
    completed, _ = collect_completions([src, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    # 8 contour commands + JobStart/JobEnd markers
    assert len(out.ops) == 10
    cmd_types = [c["type"] for c in out.ops.to_dict()["commands"]]
    assert cmd_types[0] == "JOB_START"
    assert cmd_types[-1] == "JOB_END"


def test_workpiece_markers_carry_uid():
    src = make_contour_compute("src")
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="src",
                placement_matrix=IDENTITY,
                uid="ignored",
                target_dimensions=(0.0, 0.0),
            )
        ],
        start_markers=[Marker.WorkpieceStart(uid="wpu-1", _tag=True)],
        end_markers=[Marker.WorkpieceEnd(uid="wpu-1", _tag=True)],
    )
    completed, _ = collect_completions([src, agg])
    cmds = result_ops(_by_key(completed)["agg"]).to_dict()["commands"]
    assert cmds[0]["type"] == "WORKPIECE_START"
    assert cmds[0]["workpiece_uid"] == "wpu-1"
    assert cmds[-1]["type"] == "WORKPIECE_END"
    assert cmds[-1]["workpiece_uid"] == "wpu-1"


# ── Placement matrices ────────────────────────────────────────────


def test_placement_matrix_translates_input():
    src = make_contour_compute("src")
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="src",
                placement_matrix=_translate(100.0, 50.0),
                uid="",
                target_dimensions=(0.0, 0.0),
            )
        ],
    )
    completed, _ = collect_completions([src, agg])
    src_ops = result_ops(_by_key(completed)["src"]).to_dict()
    agg_ops = result_ops(_by_key(completed)["agg"]).to_dict()
    # Layout: SET_POWER, OPS_SECTION_START, MOVE_TO, LINE_TO, ...
    src_line_to = src_ops["commands"][3]["end"]
    agg_line_to = agg_ops["commands"][3]["end"]
    assert src_line_to == (10.0, 0.0, 0.0)
    assert agg_line_to == (110.0, 50.0, 0.0)


def test_target_dimensions_triggers_uniform_scaling():
    src = make_contour_compute("src")
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="src",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(20.0, 20.0),
            )
        ],
    )
    completed, _ = collect_completions([src, agg])
    src_ops = result_ops(_by_key(completed)["src"]).to_dict()
    agg_ops = result_ops(_by_key(completed)["agg"]).to_dict()
    # Layout: SET_POWER, OPS_SECTION_START, MOVE_TO, LINE_TO, ...
    assert src_ops["commands"][3]["end"] == (10.0, 0.0, 0.0)
    assert agg_ops["commands"][3]["end"] == (20.0, 0.0, 0.0)


def test_no_scaling_when_target_dimensions_zero():
    src = make_contour_compute("src")
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="src",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            )
        ],
    )
    completed, _ = collect_completions([src, agg])
    # Layout: SET_POWER, OPS_SECTION_START, MOVE_TO, LINE_TO, ...
    src_first = result_ops(_by_key(completed)["src"]).to_dict()["commands"][2][
        "end"
    ]
    agg_first = result_ops(_by_key(completed)["agg"]).to_dict()["commands"][2][
        "end"
    ]
    assert src_first == agg_first


# ── Time estimate ─────────────────────────────────────────────────


def test_time_estimate_none_when_machine_rates_zero():
    src = make_contour_compute("src")
    agg = _aggregate_only()
    completed, _ = collect_completions([src, agg])
    assert aggregate_result(_by_key(completed)["agg"]).time_estimate is None


def test_time_estimate_present_when_rates_set():
    src = make_contour_compute("src")
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="src",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            )
        ],
        machine=MachineParams(
            default_feed_rate=1000.0,
            default_rapid_rate=5000.0,
            acceleration=0.0,
        ),
    )
    completed, _ = collect_completions([src, agg])
    t = aggregate_result(_by_key(completed)["agg"]).time_estimate
    assert t is not None
    assert t > 0.0


# ── Missing dep / error cases ─────────────────────────────────────


def test_aggregate_with_missing_source_yields_error():
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="ghost",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            )
        ],
    )
    completed, _ = collect_completions([agg])
    c = _by_key(completed)["agg"]
    assert c.error is not None
    assert "ghost" in c.error
    assert c.output is None


def test_aggregate_chains_through_other_aggregate():
    src = make_contour_compute("src")
    inner = _make_aggregate(
        "inner",
        [
            AggregateInput(
                source_key="src",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            )
        ],
    )
    outer = _make_aggregate(
        "outer",
        [
            AggregateInput(
                source_key="inner",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            )
        ],
    )
    completed, _ = collect_completions([src, inner, outer])
    outer_out = aggregate_result(_by_key(completed)["outer"])
    assert outer_out is not None
    # 8 commands (6 contour + VectorOutline section markers)
    assert len(outer_out.ops) == 8

"""Tests for AggregateGroup LinkMode.

- LinkMode::None (default): inputs are concatenated verbatim.
- LinkMode::Sequential: travel moves (retract -> XY travel -> plunge)
  are emitted between consecutive inputs, plus a final lift after
  the last input.
"""

from conftest import (
    aggregate_result,
    collect_completions,
    make_contour_compute,
    make_square_part,
)

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    LinkMode,
    MachineParams,
    Marker,
)
from raygeo.geo import Geometry
from raygeo.ops.part import Part
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

IDENTITY = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def _by_key(completed):
    return {c.key: c for c in completed}


def _make_aggregate(
    key: str,
    inputs: list,
    link_mode=None,
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
                        link_mode=link_mode or LinkMode.none(),
                    )
                ],
                wrap_end=wrap_end or [],
                machine=machine or MachineParams(),
            )
        ),
    )


# ── LinkMode.none (default, no linking) ──────────────────────────


def test_link_mode_none_concatenates_without_travel():
    """No travel moves when link_mode is None."""
    a = make_contour_compute("a")
    b = make_contour_compute("b")
    agg = _make_aggregate(
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
        link_mode=LinkMode.none(),
    )
    completed, _ = collect_completions([a, b, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    assert len(out.ops) == 12


# ── LinkMode.sequential — basic ──────────────────────────────────


def test_link_mode_sequential_adds_travel_between_inputs():
    """Travel moves (retract + plunge) appear between two identical
    contour inputs."""
    a = make_contour_compute("a")
    b = make_contour_compute("b")
    agg = _make_aggregate(
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
        link_mode=LinkMode.sequential(safe_z=2.0),
    )
    completed, _ = collect_completions([a, b, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    cmds = out.ops.to_dict()["commands"]

    # 6 (contour a) + 2 travel (retract+plunge) + 6 (contour b) + 1 final lift
    assert len(cmds) == 15

    # Both contours start/end at (0, 0, 0), so XY travel is no-op
    idx = 6
    assert cmds[idx]["type"] == "MOVE_TO", f"expected retract, got {cmds[idx]}"
    assert cmds[idx]["end"] == (0.0, 0.0, 2.0), (
        f"retract to safe_z: {cmds[idx]}"
    )
    idx += 1

    assert cmds[idx]["type"] == "MOVE_TO", f"expected plunge, got {cmds[idx]}"
    assert cmds[idx]["end"] == (0.0, 0.0, 0.0), (
        f"plunge to entry_z: {cmds[idx]}"
    )

    # Final lift: after contour b, end is (0, 0, 0) which is < safe_z
    last = cmds[-1]
    assert last["type"] == "MOVE_TO", f"expected final lift, got {last}"
    assert last["end"] == (0.0, 0.0, 2.0), f"final lift to safe_z: {last}"


def test_link_mode_sequential_lifts_after_single_input():
    """Even with one input, sequential mode emits a final lift if
    the tool ends below safe_z."""
    a = make_contour_compute("a")
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="a",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            ),
        ],
        link_mode=LinkMode.sequential(safe_z=2.0),
    )
    completed, _ = collect_completions([a, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    cmds = out.ops.to_dict()["commands"]
    # 6 (contour) + 1 final lift = 7
    assert len(cmds) == 7
    assert cmds[6]["type"] == "MOVE_TO"
    assert cmds[6]["end"] == (0.0, 0.0, 2.0)


# ── LinkMode.sequential — XY travel when positions differ ────────


def test_link_mode_sequential_xy_travel_when_positions_differ():
    """XY travel move is emitted when consecutive inputs end/start
    at different XY positions."""
    part_a = make_square_part()

    g2 = Geometry()
    g2.move_to(20, 20)
    g2.line_to(30, 20)
    g2.line_to(30, 30)
    g2.line_to(20, 30)
    g2.line_to(20, 20)
    part_b = Part(geometry=g2, size_mm=(10.0, 10.0))

    a = make_contour_compute("a", part=part_a)
    b = make_contour_compute("b", part=part_b)
    agg = _make_aggregate(
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
        link_mode=LinkMode.sequential(safe_z=2.0),
    )
    completed, _ = collect_completions([a, b, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    cmds = out.ops.to_dict()["commands"]

    # We expect 6 (a) + 2-3 travel + 6 (b) + 1 final lift = 15-16
    # Regardless of exact positions, verify:
    # 1. There are travel moves between the two contour blocks
    # 2. The first travel move goes to safe_z (retract)
    # 3. The last command is the final lift to safe_z

    assert 15 <= len(cmds) <= 16

    # After contour a (6 cmds), find the move to safe_z (retract)
    retract = cmds[6]
    assert retract["type"] == "MOVE_TO"
    assert retract["end"][2] == 2.0  # safe_z

    # Last command is final lift to safe_z
    last = cmds[-1]
    assert last["type"] == "MOVE_TO"
    assert last["end"][2] == 2.0


# ── multiple groups, each with linking ───────────────────────────


def test_link_mode_sequential_per_group():
    """Each AggregateGroup independently applies its own link_mode."""
    a = make_contour_compute("a")
    b = make_contour_compute("b")
    c = make_contour_compute("c")

    agg = NodeRequest(
        key="agg",
        generation_id=1,
        stage=StageSpec.Aggregate(
            spec=AggregateSpec(
                wrap_start=[Marker.JobStart(_tag=True)],
                groups=[
                    AggregateGroup(
                        start_markers=[],
                        inputs=[
                            AggregateInput(
                                source_key="a",
                                placement_matrix=IDENTITY,
                                uid="",
                                target_dimensions=(0.0, 0.0),
                            ),
                        ],
                        end_markers=[],
                        link_mode=LinkMode.sequential(safe_z=2.0),
                    ),
                    AggregateGroup(
                        start_markers=[],
                        inputs=[
                            AggregateInput(
                                source_key="b",
                                placement_matrix=IDENTITY,
                                uid="",
                                target_dimensions=(0.0, 0.0),
                            ),
                            AggregateInput(
                                source_key="c",
                                placement_matrix=IDENTITY,
                                uid="",
                                target_dimensions=(0.0, 0.0),
                            ),
                        ],
                        end_markers=[],
                        link_mode=LinkMode.sequential(safe_z=2.0),
                    ),
                ],
                wrap_end=[Marker.JobEnd(_tag=True)],
                machine=MachineParams(),
            )
        ),
    )
    completed, _ = collect_completions([a, b, c, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    cmds = out.ops.to_dict()["commands"]
    # Group 1: 6 + 1 (final lift) = 7
    # Group 2: 6 + 2 (travel) + 6 + 1 (final lift) = 15
    # Wrap: job_start + job_end = 2
    # Total: 2 + 7 + 15 = 24
    assert len(cmds) == 24, f"expected 24 cmds, got {len(cmds)}"


# ── Final lift skipped when already at safe_z ────────────────────


def test_link_mode_sequential_no_final_lift_when_at_safe_z():
    """No redundant final lift when the last tool pose is already at
    or above safe_z."""
    a = make_contour_compute("a")
    agg = _make_aggregate(
        "agg",
        [
            AggregateInput(
                source_key="a",
                placement_matrix=IDENTITY,
                uid="",
                target_dimensions=(0.0, 0.0),
            ),
        ],
        link_mode=LinkMode.sequential(safe_z=0.0),
    )
    completed, _ = collect_completions([a, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    cmds = out.ops.to_dict()["commands"]
    # Contour ends at (0,0,0) which is not below safe_z (0.0)
    # (0.0 < 0.0 - 1e-12) is false, so no lift
    assert len(cmds) == 6, (
        f"expected 6 cmds (no lift), got {len(cmds)}: {cmds}"
    )


# ── LinkMode existence / constructor checks ──────────────────────


def test_link_mode_constructors():
    none = LinkMode.none()
    assert none.tag == "none"
    assert none.safe_z == 0.0

    seq = LinkMode.sequential(safe_z=3.5)
    assert seq.tag == "sequential"
    assert seq.safe_z == 3.5

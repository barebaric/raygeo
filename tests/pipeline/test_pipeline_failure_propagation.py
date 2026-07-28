"""
Regression tests for failure propagation in `execute_stages`.

When a parent node produces no output (because its stage returned an
error, or because it was superseded), its dependents must NOT be
spawned. Instead, each dependent receives a synthetic ``CompletedNode``
with the reason ``"upstream failed"``, and the cascade continues
transitively through the dependency graph.

This file exercises the live Rust scheduler through the
Python-visible ``execute_stages`` entry point by constructing
``StageSpec.Compute`` nodes whose underlying assembler reliably
fails. The trigger is :class:`~raygeo.ops.assembly.frame.FrameSpec`,
which errors whenever the part's ``size_mm`` is ``(0, 0)``.
"""

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    ComputePayload,
    MachineParams,
)
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.assembly.frame import FrameSpec
from raygeo.ops.part import Part
from raygeo.pipeline.execute import clear_cache, execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

# A failed root compute node reaches the same code path as a real
# application failure: the stage's `run` returns `Err`, the node is
# reattached with an `error`, and `output_arc` is `None`.
_FAILED_PART = Part(size_mm=(0.0, 0.0))
_OK_PART = Part(size_mm=(10.0, 10.0))


def _failing_compute(key: str) -> NodeRequest:
    """A compute node whose FrameSpec assembler errors on the
    zero-size part — the resulting `error` completion propagates
    to all descendants via the new failure-cascade path."""
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=_FAILED_PART,
            params=_payload_from_spec(FrameSpec()),
        ),
        version_token=0,
    )


def _ok_compute(key: str) -> NodeRequest:
    """A compute node that succeeds: ContourSpec assembles empty Ops
    on a part with no geometry, but still returns ``Ok``."""
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=_OK_PART,
            params=_payload_from_spec(ContourSpec()),
        ),
        version_token=0,
    )


def _payload_from_spec(spec):
    """Build a minimal ComputePayload wrapping an Assembler."""
    return ComputePayload(assembler=Assembler(spec))


def _empty_aggregate(key: str, source_key: str) -> NodeRequest:
    """An aggregate node depending on a single upstream source_key.
    Carries no markers / machine params — the minimum needed for the
    scheduler to wire the dependency."""
    spec = AggregateSpec(
        wrap_start=[],
        groups=[
            AggregateGroup(
                start_markers=[],
                inputs=[
                    AggregateInput(
                        source_key=source_key,
                        placement_matrix=_IDENTITY_4X4,
                    )
                ],
                end_markers=[],
            )
        ],
        wrap_end=[],
        machine=MachineParams(),
    )
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Aggregate(spec=spec),
        version_token=0,
    )


_IDENTITY_4X4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def test_failed_compute_propagates_upstream_failed_to_dependent():
    """A failing root emits its own error; its direct dependent
    receives a synthetic ``"upstream failed"`` completion and is
    NOT spawned."""
    clear_cache()
    completed: list = []

    nodes = [
        _failing_compute("parent"),
        _empty_aggregate("child", source_key="parent"),
    ]

    execute_stages(nodes, lambda n: completed.append(n), None)

    # Two completions: one for parent, one for child. Both emitted
    # exactly once.
    assert len(completed) == 2

    by_key = {c.key: c for c in completed}
    assert by_key["parent"].output is None
    assert by_key["parent"].error is not None
    # The parent's error text comes from the Rust FrameSpec
    # assembler; just confirm it's non-empty.
    assert by_key["parent"].error.strip() != ""

    # The child must NOT have run; it must arrive as a synthetic
    # "upstream failed" error with no output.
    assert by_key["child"].output is None
    assert by_key["child"].error == "upstream failed"


def test_failure_cascades_transitively_to_grandchild():
    """The failure cascade must walk through the full dependency
    chain. A grandchild whose only path to a runnable parent is
    through a failing root also gets the synthetic
    ``"upstream failed"`` completion and is NOT spawned."""
    clear_cache()
    completed: list = []

    nodes = [
        _failing_compute("root"),
        _empty_aggregate("child", source_key="root"),
        _empty_aggregate("grandchild", source_key="child"),
    ]

    execute_stages(nodes, lambda n: completed.append(n), None)

    assert len(completed) == 3, "each node should be completed exactly once"

    by_key = {c.key: c for c in completed}
    # The root carries the real assembler error.
    assert by_key["root"].error is not None
    assert by_key["root"].error != "upstream failed"
    # Both descendants carry "upstream failed" — never spawned.
    assert by_key["child"].error == "upstream failed"
    assert by_key["child"].output is None
    assert by_key["grandchild"].error == "upstream failed"
    assert by_key["grandchild"].output is None


def test_unrelated_node_still_succeeds_when_a_sibling_fails():
    """A node that doesn't depend on the failing compute must still
    run and produce a successful completion. This guards against the
    failure cascade over-propagating to unrelated branches."""
    clear_cache()
    completed: list = []

    nodes = [
        _failing_compute("doomed"),
        _ok_compute("sibling"),
    ]

    execute_stages(nodes, lambda n: completed.append(n), None)

    assert len(completed) == 2

    by_key = {c.key: c for c in completed}
    assert by_key["doomed"].error is not None
    # The independent sibling succeeds and carries output.
    assert by_key["sibling"].error is None
    assert by_key["sibling"].output is not None


def test_successful_chain_still_runs_descendants():
    """Regression for the happy path: when no node fails, every
    descendant still spawns and produces output. This guards against
    the fix breaking normal scheduling."""
    clear_cache()
    completed: list = []

    nodes = [
        _ok_compute("parent"),
        _empty_aggregate("child", source_key="parent"),
    ]

    execute_stages(nodes, lambda n: completed.append(n), None)

    assert len(completed) == 2

    by_key = {c.key: c for c in completed}
    # Both nodes should have succeeded; no errors anywhere.
    assert by_key["parent"].error is None
    assert by_key["parent"].output is not None
    assert by_key["child"].error is None
    assert by_key["child"].output is not None

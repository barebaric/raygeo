"""Cancellation and progress-reporting tests for "longer" intents.

A "longer" intent is one with several compute leaves that take
real time. We use ``AdaptiveClearingSpec`` for that purpose: each
compute takes a few hundred milliseconds.

These tests exercise:

- mid-batch cancellation: an ``on_cancelled`` callback returns
  ``True`` after N completions; later nodes are short-circuited.
- per-node progress: ``on_progress`` events arrive in [0,1] and
  the final tick is 1.0 (or close to it when cancelled).
- batch progress: the ``on_batch_progress`` callback emits a
  fraction in (0, 1] that approaches 1.0 as nodes complete.
- cancel-propagated errors: cancelled leaves have
  ``error == "cancelled"`` and downstream aggregates report
  ``unsatisfiable dependency``.
- a 4-leaf intent completes within a sane wall-clock bound.
"""

import threading
import time

import pytest
from conftest import (
    collect_completions,
)

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    ComputePayload,
    MachineParams,
)
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.adaptive import AdaptiveClearingSpec
from raygeo.ops.part import Part
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _adaptive_part():
    boundary = [
        (-20.0, -20.0),
        (20.0, -20.0),
        (20.0, 20.0),
        (-20.0, 20.0),
    ]
    seed = [[(-5.0, -5.0), (5.0, -5.0), (5.0, 5.0), (-5.0, 5.0)]]
    return Part.from_polygons(boundary, initial=seed)


def _adaptive_node(
    key: str,
    on_progress=None,
    on_cancelled=None,
    generation_id: int = 1,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=generation_id,
        stage=StageSpec.Compute(
            part=_adaptive_part(),
            params=ComputePayload(
                assembler=Assembler(
                    AdaptiveClearingSpec(
                        tool_radius=3.0,
                        step_over=1.5,
                        target_z=-5.0,
                    )
                )
            ),
        ),
        on_progress=on_progress,
        on_cancelled=on_cancelled,
    )


IDENTITY = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def _agg(key: str, source_keys) -> NodeRequest:
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
            )
        ),
    )


def _by_key(completed):
    return {c.key: c for c in completed}


# ── Long-intent smoke test ──────────────────────────────────────


def test_four_leaf_adaptive_intent_completes_under_30_seconds():
    """A 4-leaf adaptive + aggregate intent completes within a
    sane wall-clock bound."""
    leaves = [_adaptive_node(f"leaf-{i}") for i in range(4)]
    agg = _agg("agg", [f"leaf-{i}" for i in range(4)])
    completed: list[CompletedNode] = []
    t0 = time.perf_counter()
    execute_stages(leaves + [agg], completed.append, None)
    dt = time.perf_counter() - t0
    by = _by_key(completed)
    for k in [f"leaf-{i}" for i in range(4)] + ["agg"]:
        assert k in by, f"missing {k}"
        assert by[k].error is None, f"{k} failed: {by[k].error}"
    assert dt < 30.0, f"wall-clock regression: {dt:.1f}s"


# ── Per-node progress ────────────────────────────────────────────


def test_per_node_progress_increases_monotonically():
    """For a single adaptive leaf, the on_progress fractions are
    non-decreasing and end at 1.0 (only if the run completes)."""
    events: list[tuple[float, str]] = []
    lock = threading.Lock()

    def on_progress(frac, msg):
        with lock:
            events.append((frac, msg))

    nr = _adaptive_node("k", on_progress=on_progress)
    collect_completions([nr])
    assert events, "expected at least one progress event"
    fracs = [f for f, _ in events]
    # First event should be near zero ("adaptive_clearing: assemble").
    assert fracs[0] == pytest.approx(0.0)
    # Final event should be at 1.0 ("adaptive_clearing: done").
    assert fracs[-1] == pytest.approx(1.0)
    # Fractions non-decreasing (allowing tiny float drift).
    for a, b in zip(fracs, fracs[1:]):
        assert b >= a - 1e-9, f"progress regressed: {a} -> {b}"


def test_per_node_progress_status_message_starts_and_ends():
    """The first message contains 'assemble' and the last 'done'."""
    events: list[tuple[float, str]] = []
    lock = threading.Lock()

    def on_progress(frac, msg):
        with lock:
            events.append((frac, msg))

    nr = _adaptive_node("k", on_progress=on_progress)
    collect_completions([nr])
    msgs = [m for _, m in events]
    assert msgs[0].endswith(": assemble"), msgs[0]
    assert msgs[-1].endswith(": done"), msgs[-1]


# ── Batch progress ───────────────────────────────────────────────


def test_batch_progress_ends_at_one():
    """For a 4-leaf adaptive intent, the final batch progress frac
    is exactly 1.0 (the pipeline always emits a final tick at 1.0
    once the rayon scope ends, even for cancelled batches)."""
    leaves = [_adaptive_node(f"leaf-{i}") for i in range(4)]
    agg = _agg("agg", [f"leaf-{i}" for i in range(4)])
    completed, batch = collect_completions(leaves + [agg], on_batch=True)
    fracs = [f for f, _ in batch]
    assert fracs[-1] == pytest.approx(1.0), (
        f"final batch frac not 1.0: {fracs[-1]}"
    )


def test_batch_progress_monotonic_and_end_at_one():
    """For a 4-leaf adaptive intent, batch progress fracs are
    non-decreasing and the final tick is exactly 1.0."""
    leaves = [_adaptive_node(f"leaf-{i}") for i in range(4)]
    agg = _agg("agg", [f"leaf-{i}" for i in range(4)])
    completed, batch = collect_completions(leaves + [agg], on_batch=True)
    fracs = [f for f, _ in batch]
    assert fracs[-1] == pytest.approx(1.0), (
        f"final batch frac not 1.0: {fracs[-1]}"
    )
    for a, b in zip(fracs, fracs[1:]):
        assert b >= a - 1e-9, f"batch regressed: {a} -> {b}"


def test_batch_progress_fires_more_than_once_per_node():
    """Multiple progress events fire per node in a longer run."""
    leaves = [_adaptive_node(f"leaf-{i}") for i in range(4)]
    batch: list[tuple[float, str]] = []
    completed: list[CompletedNode] = []
    execute_stages(leaves, completed.append, lambda f, m: batch.append((f, m)))
    # Each leaf fires at least 2 progress events (0.0 then 1.0).
    assert len(batch) >= 8, f"expected >=8 batch events, got {len(batch)}"


# ── Cancellation ─────────────────────────────────────────────────


def test_cancelled_leaf_completes_with_cancelled_error():
    """When on_cancelled returns True, the leaf completes with
    error='cancelled' and no output."""
    completed: list[CompletedNode] = []
    try:
        execute_stages(
            [_adaptive_node("k", on_cancelled=lambda: True)],
            completed.append,
            None,
        )
    except RuntimeError:
        pass
    assert len(completed) == 1
    c = completed[0]
    assert c.error == "cancelled"
    assert c.output is None


def test_cancel_after_one_completion_aborts_remaining():
    """Cancel returns True after the first completion; remaining
    leaves are not completed cleanly (they're either cancelled or
    receive synthetic 'unsatisfiable' completions)."""
    leaves = [_adaptive_node(f"leaf-{i}") for i in range(4)]
    completed: list[CompletedNode] = []
    cancel_state = {"count": 0, "cancel": False}

    def on_cancelled():
        cancel_state["count"] += 1
        if cancel_state["count"] > 1:
            cancel_state["cancel"] = True
        return cancel_state["cancel"]

    # Replace the per-node on_cancelled on every leaf with the counter.
    cancelled_leaves: list[NodeRequest] = []
    for leaf in leaves:
        cancelled_leaves.append(
            NodeRequest(
                key=leaf.key,
                generation_id=leaf.generation_id,
                stage=leaf.stage,
                on_cancelled=on_cancelled,
            )
        )
    try:
        execute_stages(cancelled_leaves, completed.append, None)
    except RuntimeError:
        pass
    # After cancellation, at least one node has error='cancelled' or
    # 'unsatisfiable dependency'. The batch may not have fully
    # completed.
    errors = [c.error for c in completed if c.error is not None]
    assert any("cancel" in e or "unsatisfiable" in e for e in errors), (
        f"expected a cancellation error, got {errors}"
    )


def test_cancel_propagates_to_dependent_aggregate():
    """Cancelling a leaf causes the downstream aggregate to abort
    with an 'unsatisfiable dependency' error (the leaf produced no
    output)."""
    leaves = [_adaptive_node("a", on_cancelled=lambda: True)]
    agg = _agg("agg", ["a"])
    completed: list[CompletedNode] = []
    try:
        execute_stages(leaves + [agg], completed.append, None)
    except RuntimeError:
        pass
    by = _by_key(completed)
    assert by["a"].error == "cancelled"
    assert by["agg"].error is not None
    assert (
        "missing dependency" in by["agg"].error
        or "unsatisfiable" in by["agg"].error
        or "upstream failed" in by["agg"].error
    )


def test_batch_progress_fires_even_on_cancellation():
    """Even if the batch is cancelled, the final batch progress tick
    should be exactly 1.0 (synthetic completions fill the rest)."""
    completed: list[CompletedNode] = []
    batch: list[tuple[float, str]] = []
    try:
        execute_stages(
            [_adaptive_node("k", on_cancelled=lambda: True)],
            completed.append,
            lambda f, m: batch.append((f, m)),
        )
    except RuntimeError:
        pass
    assert batch[-1][0] == pytest.approx(1.0), (
        f"final batch frac not 1.0: {batch[-1][0]}"
    )


# ── Per-node progress when cancelled ──────────────────────────────


def test_on_progress_fires_when_cancel_check_returns_false_first():
    """The early cancellation check in Compute::run polls
    on_cancelled before assemble() is invoked. Only when that poll
    returns False does progress reporting fire — the assemble
    stage then emits its 0.0 'assemble' message. This test pins
    that ordering: progress events fire on the first False→later
    transition.

    Concretely, we set on_cancelled to return False on the first
    poll (the early check), then True on subsequent polls. progress
    events should fire (at least the 0.0 'assemble').
    """
    events: list[tuple[float, str]] = []
    lock = threading.Lock()
    polls = {"n": 0}

    def on_progress(frac, msg):
        with lock:
            events.append((frac, msg))

    def on_cancelled():
        polls["n"] += 1
        # Allow the first poll to pass through. The assembler then
        # reports 0.0 progress. The next poll (during run) returns
        # True → cancelled.
        return polls["n"] > 1

    nr = _adaptive_node(
        "k", on_progress=on_progress, on_cancelled=on_cancelled
    )
    collect_completions([nr])
    # The 0.0 'assemble' event should have fired.
    assert events, "expected progress events"
    assert events[0][0] == pytest.approx(0.0)
    assert "assemble" in events[0][1]

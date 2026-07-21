"""Tests for per-node callback wiring (`on_progress`, `on_cancelled`,
`on_chunk`).

A `NodeRequest` carries three optional Python callables that the
stage fires during execution:

- `on_progress(frac: float, message: str) -> None` — fires when the
  stage reports progress; `frac` is in [0, 1].
- `on_cancelled() -> bool` — polled by the stage between meaningful
  units of work; returning `True` causes the stage to abort with
  `error="cancelled"`.
- `on_chunk(chunk: ChunkPayload) -> None` — fired by raster stages
  that emit progressive slabs. This test merely verifies wiring; no
  real raster stage is exercised here.

The aggregate batch-progress callback (`on_batch_progress`) is tested
in its own file.
"""

import threading

import pytest
from conftest import (
    collect_completions,
    make_contour_compute,
)

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    MachineParams,
)
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

IDENTITY = [
    [1.0, 0, 0, 0],
    [0, 1.0, 0, 0],
    [0, 0, 1.0, 0],
    [0, 0, 0, 1.0],
]


def _lock_collect():
    lock = threading.Lock()
    events = []
    return lock, events


# ── on_progress ────────────────────────────────────────────────────


def test_on_progress_fires_for_contour_compute():
    lock, events = _lock_collect()

    def on_progress(frac, msg):
        with lock:
            events.append((frac, msg))

    nr = make_contour_compute("k", on_progress=on_progress)
    collect_completions([nr])
    assert len(events) > 0
    for frac, _ in events:
        assert 0.0 <= frac <= 1.0


def test_on_progress_first_event_is_zero_or_low():
    lock, events = _lock_collect()

    def on_progress(frac, msg):
        with lock:
            events.append((frac, msg))

    nr = make_contour_compute("k", on_progress=on_progress)
    collect_completions([nr])
    if events:
        assert events[0][0] <= 0.5


def test_on_progress_final_event_is_one():
    lock, events = _lock_collect()

    def on_progress(frac, msg):
        with lock:
            events.append((frac, msg))

    nr = make_contour_compute("k", on_progress=on_progress)
    collect_completions([nr])
    final_fracs = [f for f, m in events if "done" in m]
    if final_fracs:
        assert max(final_fracs) == pytest.approx(1.0)


def test_on_progress_message_is_str():
    lock, events = _lock_collect()

    def on_progress(frac, msg):
        with lock:
            events.append((frac, msg))

    nr = make_contour_compute("k", on_progress=on_progress)
    collect_completions([nr])
    for _, msg in events:
        assert isinstance(msg, str)


def test_on_progress_fires_for_aggregate_group_start():
    src = make_contour_compute("src")
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
            )
        ),
    )
    lock, events = _lock_collect()
    agg_with_p = NodeRequest(
        key="agg",
        generation_id=1,
        stage=agg.stage,
        on_progress=lambda f, m: (
            lock.acquire(),
            events.append((f, m)),
            lock.release(),
        ),
    )
    collect_completions([src, agg_with_p])
    assert any("aggregate" in m for _, m in events)


# ── on_cancelled ──────────────────────────────────────────────────


def test_on_cancelled_true_aborts_compute_stage():
    collected: list[CompletedNode] = []
    try:
        execute_stages(
            [make_contour_compute("k", on_cancelled=lambda: True)],
            collected.append,
            None,
        )
    except RuntimeError:
        pass
    assert len(collected) == 1
    assert collected[0].error == "cancelled"
    assert collected[0].output is None


def test_on_cancelled_false_lets_stage_run():
    collected: list[CompletedNode] = []
    execute_stages(
        [make_contour_compute("k", on_cancelled=lambda: False)],
        collected.append,
        None,
    )
    assert len(collected) == 1
    assert collected[0].error is None
    assert collected[0].output is not None


def test_on_cancelled_raises_treated_as_not_cancelled():
    collected: list[CompletedNode] = []

    def bad_cancel():
        raise ValueError("boom")

    execute_stages(
        [make_contour_compute("k", on_cancelled=bad_cancel)],
        collected.append,
        None,
    )
    assert collected[0].error is None


def test_on_cancelled_can_be_none():
    collected: list[CompletedNode] = []
    execute_stages(
        [make_contour_compute("k", on_cancelled=None)],
        collected.append,
        None,
    )
    assert collected[0].error is None


def test_on_chunk_not_fired_for_vector_contour():
    chunks: list = []
    nr = make_contour_compute("k", on_chunk=lambda c: chunks.append(c))
    collect_completions([nr])
    assert chunks == []


def test_no_callbacks_no_error():
    nr = NodeRequest(
        key="k",
        generation_id=1,
        stage=make_contour_compute("k").stage,
    )
    completed, _ = collect_completions([nr])
    assert completed[0].error is None

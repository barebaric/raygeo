"""Pipeline stage integration tests — single-node and progress callbacks.

Exercises the Python-visible types introduced in Slice A2:
``StageSpec`` (and its variants), ``NodeRequest``, ``CompletedNode``,
and ``execute_stages`` with real Contour compute nodes.
"""

import threading

import pytest
from conftest import (
    make_contour_compute,
)

from raygeo.cnc.execution.specs import EncodeSpec
from raygeo.ops import Ops
from raygeo.ops.assembly import AssemblyOutput
from raygeo.ops.convert import Encoder, VertexSpec
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import execute_stages
from raygeo.pipeline.request import NodeRequest


def _collect(nodes, on_batch=None):
    completed: list[CompletedNode] = []
    lock = threading.Lock()

    def on_completed(node: CompletedNode) -> None:
        with lock:
            completed.append(node)

    batch_progress: list[tuple[float, str]] = []

    def _batch(frac: float, msg: str) -> None:
        batch_progress.append((frac, msg))

    execute_stages(nodes, on_completed, _batch if on_batch else None)
    return completed, batch_progress


# ── Smoke tests ───────────────────────────────────────────────────


def test_single_compute_succeeds():
    completed, _ = _collect([make_contour_compute("leaf-1")])
    assert len(completed) == 1
    c = completed[0]
    assert c.key == "leaf-1"
    assert c.error is None
    assert c.output is not None


def test_single_compute_produces_assembly_output():
    completed, _ = _collect([make_contour_compute("leaf-1")])
    out = completed[0].output
    assert out is not None
    assert isinstance(out, AssemblyOutput)
    assert hasattr(out, "ops")
    assert isinstance(out.ops, Ops)


# ── execute_stages behaviour ──────────────────────────────────────


def test_completed_node_carries_identity():
    nr = make_contour_compute("id-1")
    nr = NodeRequest(
        key=nr.key,
        generation_id=42,
        stage=nr.stage,
    )
    completed, _ = _collect([nr])
    c = completed[0]
    assert c.key == "id-1"
    assert c.generation_id == 42


def test_batch_progress_ends_at_one():
    nodes = [make_contour_compute(f"k{i}") for i in range(4)]
    _, batch = _collect(nodes, on_batch=True)
    assert batch, "expected batch progress events"
    assert batch[-1][0] == pytest.approx(1.0)
    for frac, _ in batch:
        assert 0.0 <= frac <= 1.0


def test_on_batch_progress_optional():
    completed, _ = _collect([make_contour_compute("k1")], on_batch=False)
    assert len(completed) == 1


def test_per_node_progress_callback_fires():
    progress_events: list[tuple[float, str]] = []
    lock = threading.Lock()

    def on_progress(frac: float, msg: str) -> None:
        with lock:
            progress_events.append((frac, msg))

    nr = make_contour_compute("k1", on_progress=on_progress)
    _, _ = _collect([nr])
    assert progress_events, "expected at least one progress event"
    for frac, _ in progress_events:
        assert 0.0 <= frac <= 1.0


def test_cancelled_callback_when_set():
    collected: list[CompletedNode] = []
    try:
        execute_stages(
            [make_contour_compute("k1", on_cancelled=lambda: True)],
            collected.append,
            None,
        )
    except RuntimeError:
        pass
    assert len(collected) == 1
    c = collected[0]
    assert c.error == "cancelled"
    assert c.output is None


def test_missing_dependency_yields_error():
    nodes = [
        NodeRequest(
            key="enc",
            generation_id=1,
            stage=EncodeSpec(
                source_key="does-not-exist",
                encoder=Encoder(VertexSpec()),
            ),
        )
    ]
    completed, _ = _collect(nodes)
    c = completed[0]
    assert c.error is not None
    assert "does-not-exist" in c.error


def test_execute_stages_returns_none_on_success():
    assert execute_stages([make_contour_compute("k1")], lambda n: None) is None


def test_execute_stages_raises_on_cancel():
    with pytest.raises(RuntimeError):
        execute_stages(
            [make_contour_compute("k1", on_cancelled=lambda: True)],
            lambda n: None,
        )

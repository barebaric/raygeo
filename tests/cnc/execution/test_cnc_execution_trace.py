"""Trace-related tests for the pipeline.

Tests the TraceFile reader and the Intent API together.
"""

import struct

import msgpack

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.clearing import plan_clearing
from raygeo.ops.part import Part
from raygeo.trace import TraceFile


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _completed_list():
    completed = []

    def collector(node):
        completed.append(node)

    return completed, collector


def _write_trace_file(path):
    """Write a minimal valid trace .bin file for TraceFile to read."""
    header = (
        b"RGEO"
        + struct.pack("<H", 3)
        + struct.pack("<H", 0)
        + struct.pack("<I", 3)
    )
    # Record 1: SpanStart (root) — kind=10
    root = msgpack.packb(
        {
            "kind": 10,
            "id": 1,
            "parent": 0,
            "source": "test",
            "label": "test",
        }
    )
    # Record 2: Init event — kind=12
    init = msgpack.packb(
        {
            "kind": 12,
            "seq": 0,
            "span": 1,
            "source": "test",
        }
    )
    # Record 3: SpanEnd — kind=11
    end = msgpack.packb(
        {
            "kind": 11,
            "id": 1,
            "parent": 0,
            "source": "test",
            "label": "",
        }
    )
    with open(path, "wb") as f:
        f.write(header)
        for rec in (root, init, end):
            assert rec is not None
            f.write(struct.pack("<I", len(rec)))
            f.write(rec)


def test_trace_reader_basic(tmp_path):
    """Minimal .bin file can be read by TraceFile."""
    tp = tmp_path / "trace.bin"
    _write_trace_file(tp)
    trace = TraceFile(str(tp))
    assert trace.ver == 3
    assert trace.root is not None
    assert trace.root.source == "test"
    assert trace.root.label == "test"
    assert len(trace.events) > 0


def test_trace_toolpath(tmp_path):
    """TraceFile.toolpath() returns a list."""
    tp = tmp_path / "trace.bin"
    _write_trace_file(tp)
    trace = TraceFile(str(tp))
    tp_result = trace.toolpath()
    assert isinstance(tp_result, list)


def test_trace_len_and_getitem(tmp_path):
    """TraceFile supports __len__ and __getitem__."""
    tp = tmp_path / "trace.bin"
    _write_trace_file(tp)
    trace = TraceFile(str(tp))
    assert len(trace) >= 1
    assert trace[0] is not None


def test_intent_with_clear_plan_produces_completions():
    """run_intent returns Ops and invokes on_completed for each node."""
    boundary = _rect(-10, -10, 20, 20)
    part = Part.from_polygons(boundary, [], (0.0, 0.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    assert plan.step_count >= 1

    intent = create_intent(plan, part, 0)
    completed, on_completed = _completed_list()
    ops = run_intent(intent, on_completed=on_completed)
    assert ops is not None
    assert len(completed) >= 2
    for node in completed:
        assert node.key
        assert (node.output is not None) or (node.error is not None)

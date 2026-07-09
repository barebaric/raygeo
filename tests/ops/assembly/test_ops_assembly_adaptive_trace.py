"""Tests for adaptive-clearing trace generation (new span/event format).

SEMANTIC MAPPING (old -> new format):

  - old init/cut/exit record kinds
      -> EventKind Init / Move(move_kind=cut) / Exit
  - old top-level "geometry" record
      -> span.attrs on the adaptive span
         (tool_radius, boundary, islands, seeds)
  - old pos_x/pos_y/heading fields
      -> Event.tool.{pos_x,pos_y,heading}
  - old step_idx
      -> Event.progress.step_idx
  - old resume_stall/resume_stuck
      -> Event with kind "resume" + meta
  - trace file version
      -> 3 (was 2)

These tests drive the assembler via a Workplan (FlatSpiral seed +
AdaptiveClear step) and read back the trace file written by
``Workplan.execute(trace=path)``.
"""

import math
import struct

import pytest

from raygeo.cnc.machining.plan import Workplan
from raygeo.ops.assembly.adaptive import (
    ResumePointNotFoundError,
)
from raygeo.trace import TraceFile


def _circle(cx, cy, r, n=32):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _run_adaptive(boundary, islands=None, trace_path=None):
    """Run adaptive clearing via a Workplan and return the AssemblyResult.

    Deposits a FlatSpiral seed disk (r=5) so adaptive has engagement to
    start from, then clears the pocket with AdaptiveClear.
    """
    islands = islands or []
    steps = [
        {
            "kind": "FlatSpiral",
            "center": (0.0, 0.0),
            "z": -5.0,
            "start_radius": 0.0,
            "end_radius": 5.0,
            "revolutions": 2.0,
            "direction": "CW",
            "angular_step": 0.1,
            "start_angle": 0.0,
        },
        {
            "kind": "AdaptiveClear",
            "pocket_boundary": boundary,
            "islands": islands,
            "tool_radius": 3.0,
            "step_over": 1.5,
            "step_length": 0.6,
            "target_z": -5.0,
            "safe_z": 2.0,
            "max_deflection_deg": 30.0,
            "wall_margin": 0.0,
            "area_tolerance": 4.0,
            "angular_step": 0.1,
        },
    ]
    wp = Workplan(boundary, islands=islands or None, safe_z=2.0)
    wp.extend(steps)
    return wp.execute(trace=trace_path)


# ── No trace when disabled ─────────────────────────────────────────────


def test_trace_disabled(tmp_path):
    """When trace_path is None, no trace file is created."""
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=None)
    assert list(tmp_path.iterdir()) == []


def test_trace_no_file_without_path(tmp_path):
    """Ensuring no spurious trace file when trace_path omitted."""
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary)
    assert list(tmp_path.iterdir()) == []


# ── File creation + header ─────────────────────────────────────────────


def test_trace_creates_file(tmp_path):
    """When trace_path is given, a binary trace file is created."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    assert (tmp_path / "trace.bin").exists()


def test_trace_valid_header(tmp_path):
    """Trace file has correct RGEO magic and version 3."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    with open(tp, "rb") as f:
        magic = f.read(4)
        ver = struct.unpack("<H", f.read(2))[0]
    assert magic == b"RGEO"
    assert ver == 3


# ── Record structure ───────────────────────────────────────────────────


def test_trace_has_records(tmp_path):
    """Trace contains init, at least one cut move, and an exit event."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    kinds = {e.kind for e in trace.events}
    assert "init" in kinds
    assert "exit" in kinds
    move_kinds = {e.move_kind for e in trace.events if e.kind == "move"}
    assert "cut" in move_kinds


def test_trace_file_readable(tmp_path):
    """TraceFile exposes ver, spans, events."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    assert trace.ver == 3
    assert len(trace.events) >= 3
    assert trace.root is not None


def test_trace_has_adaptive_span(tmp_path):
    """The trace contains a span sourced from the adaptive assembler."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    assert "adaptive" in trace.sources
    adaptive_spans = [s for s in trace.spans if s.source == "adaptive"]
    assert adaptive_spans, "no adaptive span in trace"


# ── Geometry / setup attrs (replaces old "geometry" record) ────────────


def test_trace_geometry_attrs(tmp_path):
    """The adaptive span attrs hold tool_radius, boundary, and islands."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    islands = [_rect(5, 0, 6, 6)]
    _run_adaptive(boundary, islands=islands, trace_path=tp)
    trace = TraceFile(tp)
    span = next(s for s in trace.spans if s.source == "adaptive")
    attrs = span.attrs
    assert attrs["tool_radius"] == pytest.approx(3.0)
    assert len(attrs["boundary"]) == 4
    assert len(attrs["islands"]) == 1
    assert len(attrs["islands"][0]) == 4


# ── Positions ──────────────────────────────────────────────────────────


def test_trace_cut_records_have_positions(tmp_path):
    """Cut moves carry valid pos_x/pos_y within pocket bounds."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    cuts = [
        e for e in trace.events if e.kind == "move" and e.move_kind == "cut"
    ]
    assert cuts, "no cut moves in trace"
    for e in cuts:
        assert e.tool is not None
        assert not math.isnan(e.tool.pos_x)
        assert not math.isnan(e.tool.pos_y)
        assert -18 <= e.tool.pos_x <= 18
        assert -18 <= e.tool.pos_y <= 18


# ── step_idx monotonicity ──────────────────────────────────────────────


def test_trace_records_have_step_idx(tmp_path):
    """Move/resume events carry a strictly increasing step_idx.

    Events from the adaptive span only — other steps (e.g. the FlatSpiral
    seed) have their own step_idx starting from 0.
    """
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    adaptive_span = next(s for s in trace.spans if s.source == "adaptive")
    idx = [
        e.progress.step_idx
        for e in adaptive_span.events
        if e.progress is not None and e.kind in ("move", "resume")
    ]
    assert len(idx) >= 1
    for a, b in zip(idx, idx[1:]):
        assert a < b, f"step_idx not increasing: {a} >= {b}"


# ── Islands ────────────────────────────────────────────────────────────


def test_trace_with_islands(tmp_path):
    """Island too close to wall creates unreachable material.

    The 6x6 island centered at (8,0) leaves only 4 mm between its right
    edge (x=11) and the right wall (x=15).  The tool (radius 3 mm) cannot
    fit through a 4 mm gap (2R = 6 mm), so a region on the far side is
    topologically unreachable.  After clearing the reachable region the
    stepper raises ``ResumePointNotFoundError``.  This is the expected
    outcome, not a bug.

    The trace file is written before the error propagates (the Workplan's
    Tracer flushes on Drop), so we can still read it.  The adaptive span
    will NOT be present because the error occurred before its events were
    emitted into the Workplan's tracer; we check the root span's attrs
    instead.
    """
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    islands = [_rect(8, 0, 6, 6)]
    with pytest.raises(ResumePointNotFoundError):
        _run_adaptive(boundary, islands=islands, trace_path=tp)
    trace = TraceFile(tp)
    span = next(s for s in trace.spans if s.source == "workplan")
    assert len(span.attrs["islands"]) == 1
    # Check cut events from the adaptive span (if any — it may be absent
    # because the error occurred before the Workplan wrote its events).
    adaptive_span = next(
        (s for s in trace.spans if s.source == "adaptive"), None
    )
    if adaptive_span is not None:
        for e in adaptive_span.events:
            if e.kind == "move" and e.move_kind == "cut":
                assert e.tool is not None
                x, y = e.tool.pos_x, e.tool.pos_y
                in_island = 5 <= x <= 11 and -3 <= y <= 3
                assert not in_island, (
                    f"cut position ({x:.1f}, {y:.1f}) inside island"
                )

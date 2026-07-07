"""Tests for trace file generation in adaptive clearing."""

import math
import struct

import pytest

from raygeo.ops.assembly.adaptive import (
    ResumePointNotFoundError,
    adaptive_clearing,
)
from raygeo.ops.cut.cleared_area import ClearedArea
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
    """Run adaptive clearing from a circle seed and return the Ops."""
    islands = islands or []
    seed = [_circle(0, 0, 5)]
    ca = ClearedArea(boundary=boundary, islands=islands, initial=seed)
    clear_result = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        area_tolerance=4.0,
        trace_path=trace_path,
    )
    return clear_result.ops


def test_trace_disabled(tmp_path):
    """When trace_path is None, no trace file is created."""
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=None)
    assert list(tmp_path.iterdir()) == []


def test_trace_creates_file(tmp_path):
    """When trace_path is given, a binary trace file is created."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    assert (tmp_path / "trace.bin").exists()


def test_trace_valid_header(tmp_path):
    """Trace file has correct RGEO magic and version."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    with open(tp, "rb") as f:
        magic = f.read(4)
        ver = struct.unpack("<H", f.read(2))[0]
    assert magic == b"RGEO"
    assert ver == 2


def test_trace_has_records(tmp_path):
    """Trace file contains at least init, one cut, and an exit record."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    kinds = [trace[i]["kind"] for i in range(len(trace))]
    assert "init" in kinds
    assert "cut" in kinds
    assert "exit" in kinds


def test_trace_geometry_record(tmp_path):
    """Geometry record holds correct tool_radius, boundary, islands."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    islands = [_rect(5, 0, 6, 6)]
    _run_adaptive(boundary, islands=islands, trace_path=tp)
    trace = TraceFile(tp)
    geo = trace.geometry
    assert geo["kind"] == "geometry"
    assert geo["tool_radius"] == 3.0
    assert len(geo["boundary"]) == 4
    assert len(geo["islands"]) == 1
    assert len(geo["islands"][0]) == 4


def test_trace_file_readable(tmp_path):
    """TraceFile supports len, __getitem__, and ver."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    assert trace.ver == 2
    assert len(trace) >= 3
    rec = trace[0]
    assert rec["kind"] is not None
    assert rec["kind"] == "init" or rec["kind"] == "geometry"


def test_trace_cut_records_have_positions(tmp_path):
    """Cut records contain valid pos_x/pos_y within pocket bounds."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    for i in range(len(trace)):
        rec = trace[i]
        if rec.kind == "cut":
            pos_x = rec["pos_x"]
            pos_y = rec["pos_y"]
            assert not math.isnan(pos_x)
            assert not math.isnan(pos_y)
            # Should be within pocket bounds (with margin for tool radius)
            assert -18 <= pos_x <= 18
            assert -18 <= pos_y <= 18


def test_trace_no_file_without_path(tmp_path):
    """Ensuring no spurious trace file when trace_path omitted."""
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary)
    assert list(tmp_path.iterdir()) == []


def test_trace_records_have_step_idx(tmp_path):
    """Cut and resume records have an increasing step_idx."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_adaptive(boundary, trace_path=tp)
    trace = TraceFile(tp)
    indices = []
    for i in range(len(trace)):
        rec = trace[i]
        if rec.kind in ("cut", "resume_stall", "resume_stuck"):
            indices.append(rec["step_idx"])
    assert len(indices) >= 1
    # Indices should be strictly increasing
    for a, b in zip(indices, indices[1:]):
        assert a < b


def test_trace_with_islands(tmp_path):
    """Island too close to wall creates unreachable material.

    The 6×6 island centered at (8,0) leaves only 4 mm between its
    right edge (x=11) and the right wall (x=15).  The tool
    (radius 3 mm) cannot fit through a 4 mm gap (2R = 6 mm), so a
    region on the far side is topologically unreachable.

    After clearing the reachable region, the stepper correctly detects
    no remaining engagement and raises `ResumePointNotFoundError`.
    This is *not* a bug — it is the expected outcome for a geometry
    with narrow passages the tool cannot navigate.
    """
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    islands = [_rect(8, 0, 6, 6)]
    with pytest.raises(ResumePointNotFoundError):
        _run_adaptive(boundary, islands=islands, trace_path=tp)
    trace = TraceFile(tp)
    geo = trace.geometry
    assert len(geo["islands"]) == 1
    # Toolpath positions should not be inside the island
    for i in range(len(trace)):
        rec = trace[i]
        if rec.kind == "cut":
            x = rec["pos_x"]
            y = rec["pos_y"]
            in_island = 5 <= x <= 11 and -3 <= y <= 3
            assert not in_island, (
                f"cut position ({x:.1f}, {y:.1f}) inside island"
            )

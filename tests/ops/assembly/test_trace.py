"""Tests for trace file generation in adaptive clearing."""

import math
import struct

from raygeo.ops import Ops
from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.assembly.entry import adaptive_entry
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.trace import TraceFile


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _run_adaptive(boundary, islands=None, trace_path=None):
    """Run adaptive entry + clearing and return the combined Ops."""
    islands = islands or []
    entry_ops, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
    )
    ca = ClearedArea(boundary=boundary, islands=islands, initial=cp)
    clear_ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
        area_tolerance=4.0,
        trace_path=trace_path,
    )
    combined = Ops()
    combined.extend(entry_ops)
    combined.extend(clear_ops)
    return combined


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
            assert -2 <= pos_x <= 32
            assert -2 <= pos_y <= 32


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
    """Trace with islands succeeds and records reflect it."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    islands = [_rect(0, 0, 6, 6)]
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
            in_island = -3 <= x <= 3 and -3 <= y <= 3
            assert not in_island, (
                f"cut position ({x:.1f}, {y:.1f}) inside island"
            )

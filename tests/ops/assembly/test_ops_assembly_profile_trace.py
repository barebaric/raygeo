"""Tests for trace file generation in profile operations."""

import math
import struct
from typing import Any

from raygeo.ops import Ops
from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.trace import TraceFile


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _kwargs(
    boundary, islands=None, trace_path=None, **overrides: Any
) -> dict[str, Any]:
    """Build a kwargs dict for profile_outer/profile_inner."""
    kw = dict(
        boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        stock_to_leave=0.0,
        cut_feed_rate=1200,
        cut_power=0.5,
        trace_path=trace_path,
    )
    if islands is not None:
        kw["islands"] = islands
    kw.update(overrides)
    return kw


def _run_outer(boundary, trace_path=None, **overrides: Any) -> Ops:
    ca = ClearedArea(boundary=boundary, initial=[_rect(0, 0, 8, 8)])
    kw = _kwargs(boundary, trace_path=trace_path, **overrides)
    result = profile_outer(ca, **kw)
    return result.ops


def _run_inner(
    boundary, islands=None, trace_path=None, **overrides: Any
) -> Ops:
    ca = ClearedArea(
        boundary=boundary, islands=islands or [], initial=[_rect(0, 0, 8, 8)]
    )
    kw = _kwargs(boundary, islands=islands, trace_path=trace_path, **overrides)
    result = profile_inner(ca, **kw)
    return result.ops


# ── Trace disabled ─────────────────────────────────────────────────────


def test_profile_outer_trace_disabled_by_default(tmp_path):
    """When trace_path is None, no trace file is created."""
    boundary = _rect(0, 0, 30, 30)
    _run_outer(boundary, trace_path=None)
    assert list(tmp_path.iterdir()) == []


# ── Trace creates file ─────────────────────────────────────────────────


def test_profile_outer_trace_creates_file(tmp_path):
    """When trace_path is given, a binary trace file is created."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_outer(boundary, trace_path=tp)
    assert (tmp_path / "trace.bin").exists()


# ── Valid header ───────────────────────────────────────────────────────


def test_profile_outer_trace_valid_header(tmp_path):
    """Trace file has correct RGEO magic and version."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_outer(boundary, trace_path=tp)
    with open(tp, "rb") as f:
        magic = f.read(4)
        ver = struct.unpack("<H", f.read(2))[0]
    assert magic == b"RGEO"
    assert ver == 2


# ── Records exist ──────────────────────────────────────────────────────


def test_profile_outer_trace_has_records(tmp_path):
    """Trace file contains init, polygon_start, cut, polygon_end, exit."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_outer(boundary, trace_path=tp)
    trace = TraceFile(tp)
    kinds = [trace[i]["kind"] for i in range(len(trace))]
    assert "init" in kinds
    assert "polygon_start" in kinds
    assert "cut" in kinds
    assert "polygon_end" in kinds
    assert "exit" in kinds
    # Order: init, geometry, polygon_start, (cut)*, polygon_end, exit.
    # geometry is always first, so check init comes before cut.
    init_idx = kinds.index("init")
    cut_idx = kinds.index("cut")
    assert init_idx < cut_idx


# ── Geometry record ────────────────────────────────────────────────────


def test_profile_outer_trace_geometry_record_has_offset_polys(tmp_path):
    """Geometry record has offset_polys and walk_order keys."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_outer(boundary, trace_path=tp)
    trace = TraceFile(tp)
    geo = trace.geometry
    assert geo["kind"] == "geometry"
    assert "offset_polys" in geo
    assert "walk_order" in geo
    assert len(geo["offset_polys"]) >= 1
    # Offset polygon is linearized (round joins → many vertices)
    assert len(geo["offset_polys"][0]) >= 4


# ── step_idx increasing ────────────────────────────────────────────────


def test_profile_outer_trace_records_have_increasing_step_idx(tmp_path):
    """Cut records have a strictly increasing step_idx."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_outer(boundary, trace_path=tp)
    trace = TraceFile(tp)
    indices = []
    for i in range(len(trace)):
        rec = trace[i]
        if rec.kind in ("cut", "polygon_start", "polygon_end"):
            indices.append(rec["step_idx"])
    assert len(indices) >= 2
    for a, b in zip(indices, indices[1:]):
        assert a < b, f"step_idx not increasing: {a} >= {b}"


# ── Valid positions ────────────────────────────────────────────────────


def test_profile_outer_trace_cut_records_have_valid_positions(tmp_path):
    """Cut records have valid pos_x/pos_y within offset bounds."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 30, 30)
    _run_outer(boundary, trace_path=tp)
    trace = TraceFile(tp)
    for i in range(len(trace)):
        rec = trace[i]
        if rec.kind == "cut":
            pos_x = rec["pos_x"]
            pos_y = rec["pos_y"]
            assert not math.isnan(pos_x)
            assert not math.isnan(pos_y)
            # offset polygon is ~66×66 for 30×30 boundary with radius 3
            # (30 + 2*3*1.414 + 3 = ~42 each side from center)
            assert -45 <= pos_x <= 45
            assert -45 <= pos_y <= 45


# ── Inner trace with islands ───────────────────────────────────────────


def test_profile_inner_trace_has_polygon_starts_for_outer_and_each_island(
    tmp_path,
):
    """Inner profile emits polygon_start for each polygon walked."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 60, 60)
    # Two accessible islands
    islands = [
        _rect(-10, 0, 10, 10),
        _rect(10, 0, 10, 10),
    ]
    _run_inner(boundary, islands=islands, trace_path=tp)
    trace = TraceFile(tp)
    starts = []
    for i in range(len(trace)):
        rec = trace[i]
        if rec.kind == "polygon_start":
            starts.append(rec)
    # At least outer (idx 0) + two islands
    assert len(starts) >= 3
    indices = [s["target_polygon_idx"] for s in starts]
    assert 0 in indices
    assert 1 in indices
    assert 2 in indices


# ── No spurious file ───────────────────────────────────────────────────


def test_profile_outer_no_file_without_path(tmp_path):
    """Ensuring no spurious trace file when trace_path omitted."""
    boundary = _rect(0, 0, 30, 30)
    _run_outer(boundary)
    assert list(tmp_path.iterdir()) == []


# ── feed_change records (engagement adaptation) ────────────────────────


def test_profile_outer_trace_feed_change_on_engagement(tmp_path):
    """Feed change records appear when engagement triggers adaptation.

    Uses stock_to_leave > 0 to activate the engagement check,
    a small initial seed, and low engagement thresholds to
    provoke reductions.
    """
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 60, 60)
    # Small seed + low threshold + stock_to_leave gate the engagement check on
    ca = ClearedArea(boundary=boundary, initial=[_rect(0, 0, 6, 6)])
    kw = _kwargs(
        boundary,
        trace_path=tp,
        stock_to_leave=1.0,
        engagement_area_threshold=1.0,
        engagement_angle_threshold=0.5,
    )
    profile_outer(ca, **kw)
    trace = TraceFile(tp)
    feed_changes = [
        trace[i] for i in range(len(trace)) if trace[i].kind == "feed_change"
    ]
    # Engagement should fire on a fully-buried tool
    assert len(feed_changes) >= 1, "expected at least one feed_change record"
    # At least one record should show a changed feed rate
    has_change = any(r["current_feed_rate"] != 1200 for r in feed_changes)
    assert has_change, "feed change records exist but all show the same rate"

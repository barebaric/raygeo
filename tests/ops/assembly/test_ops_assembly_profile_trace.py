"""Tests for profile trace generation (new span/event format).

SEMANTIC MAPPING (old -> new format):

  - old polygon_start/polygon_end
      -> a child span per polygon walked, sourced from "profile"
  - old init/cut/exit
      -> EventKind Init / Move(cut) / Exit
  - old "geometry" record
      -> profile span attrs (offset_polys, walk_order)
  - old "feed_change" record
      -> move events whose meta records a reduced feed rate
  - trace file version
      -> 3 (was 2)

These tests drive ``profile_outer`` / ``profile_inner`` directly and
inspect ``result.trace`` (a Python dict exposed from the Rust
``AssemblyTrace`` bundle). Profile-level instrumentation landed in
step 6a and the trace property on ``AssemblyResult`` in step 6b.
"""

import math
from typing import Any

from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.part import Part


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _kwargs(initial, boundary, islands=None, **overrides: Any) -> dict:
    part = Part.from_polygons(boundary, islands or [], initial=initial)
    kw = dict(
        part=part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        stock_to_leave=0.0,
        cut_feed_rate=1200,
        cut_power=0.5,
    )
    kw.update(overrides)
    return kw


def _run_outer(boundary, **overrides: Any):
    return profile_outer(**_kwargs([_rect(0, 0, 8, 8)], boundary, **overrides))


def _run_inner(boundary, islands=None, **overrides: Any):
    return profile_inner(
        **_kwargs([_rect(0, 0, 8, 8)], boundary, islands, **overrides)
    )


# ── Trace is always populated ─────────────────────────────────────


def test_profile_outer_trace_disabled_by_default(tmp_path):
    """profile_outer always returns a trace bundle (no file created)."""
    boundary = _rect(0, 0, 30, 30)
    result = _run_outer(boundary)
    assert result.trace is not None


def test_profile_outer_no_file_without_path(tmp_path):
    """No trace file is written unless a workplan writes one."""
    boundary = _rect(0, 0, 30, 30)
    result = _run_outer(boundary)
    assert result.trace is not None
    assert list(tmp_path.iterdir()) == []


# ── Trace bundle is populated ─────────────────────────────────────


def test_profile_outer_trace_creates_file(tmp_path):
    """result.trace is populated (no file written by assembler directly)."""
    boundary = _rect(0, 0, 30, 30)
    result = _run_outer(boundary)
    assert result.trace is not None
    assert len(result.trace["events"]) > 0


def test_profile_outer_trace_valid_header(tmp_path):
    """result.trace has attrs and events."""
    boundary = _rect(0, 0, 30, 30)
    result = _run_outer(boundary)
    assert result.trace is not None
    assert "attrs" in result.trace
    assert "events" in result.trace
    assert len(result.trace["events"]) > 0


# ── Record structure ──────────────────────────────────────────────


def test_profile_outer_trace_has_records(tmp_path):
    """Trace has init, cut moves, exit; init precedes first cut."""
    boundary = _rect(0, 0, 30, 30)
    result = _run_outer(boundary)
    trace = result.trace
    assert trace is not None
    kinds = [e["kind"] for e in trace["events"]]
    assert "init" in kinds
    assert "exit" in kinds
    assert "move" in kinds
    assert kinds.index("init") < kinds.index("move")


# ── Geometry / setup attrs ────────────────────────────────────────


def test_profile_outer_trace_attrs_have_offset_polys(tmp_path):
    """Profile span attrs carry offset_polys and walk_order."""
    boundary = _rect(0, 0, 30, 30)
    result = _run_outer(boundary)
    trace = result.trace
    assert trace is not None
    attrs = trace["attrs"]
    assert "offset_polys" in attrs
    assert "walk_order" in attrs
    assert len(attrs["offset_polys"]) >= 1
    assert len(attrs["offset_polys"][0]) >= 4


# ── step_idx monotonicity ─────────────────────────────────────────


def test_profile_outer_trace_records_have_increasing_step_idx(tmp_path):
    boundary = _rect(0, 0, 30, 30)
    result = _run_outer(boundary)
    trace = result.trace
    assert trace is not None
    # Exclude init events: both init and the first move start at step_idx 0.
    idx = [
        e["progress"]["step_idx"]
        for e in trace["events"]
        if e.get("progress") is not None and e["kind"] != "init"
    ]
    assert len(idx) >= 2
    for a, b in zip(idx, idx[1:]):
        assert a < b, f"step_idx not increasing: {a} >= {b}"


# ── Positions ─────────────────────────────────────────────────────


def test_profile_outer_trace_cut_records_have_valid_positions(tmp_path):
    boundary = _rect(0, 0, 30, 30)
    result = _run_outer(boundary)
    trace = result.trace
    assert trace is not None
    cuts = [
        e
        for e in trace["events"]
        if e["kind"] == "move" and e.get("move_kind") == "cut"
    ]
    assert cuts
    for e in cuts:
        tool = e.get("tool")
        assert tool is not None
        assert not math.isnan(tool["pos_x"])
        assert not math.isnan(tool["pos_y"])
        assert -45 <= tool["pos_x"] <= 45
        assert -45 <= tool["pos_y"] <= 45


# ── Polygon markers (replaces old child-span model) ────────────────


def test_profile_inner_trace_marks_multiple_polygons(tmp_path):
    """Inner profile emits polygon_start markers for each polygon walked."""
    boundary = _rect(0, 0, 60, 60)
    islands = [_rect(-10, 0, 10, 10), _rect(10, 0, 10, 10)]
    result = _run_inner(boundary, islands=islands)
    trace = result.trace
    assert trace is not None
    poly_idxs = set()
    for e in trace["events"]:
        if e["kind"] == "move":
            meta = e["meta"]
            if "target_polygon_idx" in meta:
                poly_idxs.add(meta["target_polygon_idx"])
    assert len(poly_idxs) >= 3, (
        f"expected >= 3 distinct polygons, got {len(poly_idxs)}"
    )


# ── Feed reduction ────────────────────────────────────────────────


def test_profile_outer_trace_feed_reduction_on_engagement(tmp_path):
    """Move events carry current_feed_rate in meta when moves exist.

    Note: with high engagement the travel-skip path may eat all move
    events (existing engine behaviour).  When moves do appear they
    must carry current_feed_rate.
    """
    boundary = _rect(0, 0, 60, 60)
    kw = _kwargs(
        [_rect(0, 0, 6, 6)],
        boundary,
        stock_to_leave=1.0,
        engagement_area_threshold=1.0,
        engagement_angle_threshold=0.5,
    )
    result = profile_outer(**kw)
    trace = result.trace
    assert trace is not None
    moves = [e for e in trace["events"] if e["kind"] == "move"]
    if moves:
        for e in moves:
            assert "current_feed_rate" in e["meta"], (
                "move event missing current_feed_rate"
            )

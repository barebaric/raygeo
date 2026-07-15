"""Tests for material test grid trace generation.

These tests verify that the material test grid assembler produces a
complete trace bundle with attrs and events, and that the events carry
the expected cell metadata.
"""

import math

from raygeo.ops.assembly.material_test_grid import generate_material_test_grid


def _run_grid(
    size_mm: tuple[float, float] = (200.0, 200.0),
    cols: int = 3,
    rows: int = 3,
    **kwargs,
):
    return generate_material_test_grid(
        size_mm=size_mm, cols=cols, rows=rows, **kwargs
    )


# ── Trace is always populated ─────────────────────────────────────


def test_material_grid_trace_is_populated():
    result = _run_grid()
    assert result.trace is not None


def test_material_grid_trace_has_events():
    result = _run_grid()
    trace = result.trace
    assert trace is not None
    assert len(trace["events"]) > 0


def test_material_grid_trace_has_attrs():
    result = _run_grid()
    trace = result.trace
    assert trace is not None
    assert "attrs" in trace
    assert "events" in trace


# ── Record structure ──────────────────────────────────────────────


def test_material_grid_trace_has_init_and_exit():
    result = _run_grid()
    trace = result.trace
    assert trace is not None
    kinds = [e["kind"] for e in trace["events"]]
    assert "init" in kinds
    assert "exit" in kinds


def test_material_grid_trace_init_precedes_exit():
    result = _run_grid()
    trace = result.trace
    assert trace is not None
    kinds = [e["kind"] for e in trace["events"]]
    assert kinds.index("init") < kinds.index("exit")


def test_material_grid_trace_has_cut_moves():
    result = _run_grid()
    trace = result.trace
    assert trace is not None
    kinds = [e["kind"] for e in trace["events"]]
    assert "move" in kinds
    cuts = [
        e
        for e in trace["events"]
        if e["kind"] == "move" and e.get("move_kind") == "cut"
    ]
    assert len(cuts) > 0


# ── Attributes ────────────────────────────────────────────────────


def test_material_grid_trace_attrscontain_grid_config():
    result = _run_grid(cols=4, rows=5, grid_mode="Power vs Speed")
    trace = result.trace
    assert trace is not None
    attrs = trace["attrs"]
    assert attrs["cols"] == 4
    assert attrs["rows"] == 5
    assert attrs["grid_mode"] == "Power vs Speed"


def test_material_grid_trace_attrs_have_speed_range():
    result = _run_grid(min_speed=100.0, max_speed=500.0)
    trace = result.trace
    assert trace is not None
    attrs = trace["attrs"]
    assert attrs["min_speed"] == 100.0
    assert attrs["max_speed"] == 500.0


def test_material_grid_trace_attrs_have_power_range():
    result = _run_grid(min_power=10.0, max_power=80.0)
    trace = result.trace
    assert trace is not None
    attrs = trace["attrs"]
    assert attrs["min_power"] == 10.0
    assert attrs["max_power"] == 80.0


def test_material_grid_trace_attrs_have_offset_range():
    result = _run_grid(
        grid_mode="Speed vs Offset", min_offset=-1.0, max_offset=1.0
    )
    trace = result.trace
    assert trace is not None
    attrs = trace["attrs"]
    assert attrs["min_offset"] == -1.0
    assert attrs["max_offset"] == 1.0


def test_material_grid_trace_attrs_have_mode():
    result = _run_grid(mode="cut")
    trace = result.trace
    assert trace is not None
    attrs = trace["attrs"]
    assert attrs["mode"] == "cut"


# ── Cell metadata in events ───────────────────────────────────────


def test_material_grid_cut_events_have_cell_metadata():
    result = _run_grid(cols=2, rows=2)
    trace = result.trace
    assert trace is not None
    cuts = [
        e
        for e in trace["events"]
        if e["kind"] == "move"
        and e.get("move_kind") == "cut"
        and e.get("meta")
    ]
    assert len(cuts) > 0
    for e in cuts:
        meta = e["meta"]
        assert "cell_idx" in meta
        assert "col" in meta
        assert "row" in meta
        assert "speed" in meta
        assert "power" in meta
        assert "passes" in meta


def test_material_grid_cut_events_have_valid_positions():
    result = _run_grid(cols=2, rows=2)
    trace = result.trace
    assert trace is not None
    cuts = [
        e
        for e in trace["events"]
        if e["kind"] == "move" and e.get("move_kind") == "cut"
    ]
    for e in cuts:
        tool = e.get("tool")
        assert tool is not None
        assert not math.isnan(tool["pos_x"])
        assert not math.isnan(tool["pos_y"])


def test_material_grid_cell_idx_monotonic():
    result = _run_grid(cols=2, rows=2)
    trace = result.trace
    assert trace is not None
    cuts = [
        e
        for e in trace["events"]
        if e["kind"] == "move"
        and e.get("move_kind") == "cut"
        and e.get("meta")
    ]
    indices = [e["meta"]["cell_idx"] for e in cuts]
    # cell_idx is constant within each cell, then increments.
    # All values should be in 0..num_cells and non-decreasing.
    assert indices == sorted(indices)
    assert indices[0] == 0
    assert max(indices) == 3  # 2x2 grid → 4 cells (0..3)


def test_material_grid_total_cells_in_init():
    result = _run_grid(cols=3, rows=4)
    trace = result.trace
    assert trace is not None
    init_events = [e for e in trace["events"] if e["kind"] == "init"]
    assert len(init_events) == 1
    meta = init_events[0]["meta"]
    assert meta["total_cells"] == 12  # 3 * 4


# ── Speed vs Offset mode ──────────────────────────────────────────


def test_material_grid_speed_vs_offset_power_scales_with_speed():
    """Power should scale up with speed so darkness stays comparable."""
    result = _run_grid(
        cols=3,
        rows=1,
        grid_mode="Speed vs Offset",
        min_speed=500.0,
        max_speed=1500.0,
        fixed_power=20.0,
    )
    trace = result.trace
    assert trace is not None
    cuts = [
        e
        for e in trace["events"]
        if e["kind"] == "move"
        and e.get("move_kind") == "cut"
        and e.get("meta")
    ]
    by_col = {}
    for e in cuts:
        by_col.setdefault(e["meta"]["col"], e["meta"])
    assert by_col[0]["power"] == 20.0
    assert by_col[2]["power"] > by_col[0]["power"]


def test_material_grid_speed_vs_offset_shifts_geometry_by_row():
    """Each row's cut geometry should shift in X by its offset value."""
    result = _run_grid(
        cols=1,
        rows=3,
        grid_mode="Speed vs Offset",
        min_offset=-0.5,
        max_offset=0.5,
        shape_size=20.0,
    )
    trace = result.trace
    assert trace is not None
    cuts = [
        e
        for e in trace["events"]
        if e["kind"] == "move"
        and e.get("move_kind") == "cut"
        and e.get("meta")
    ]
    min_x_by_row = {}
    for e in cuts:
        row = e["meta"]["row"]
        x = e["tool"]["pos_x"]
        min_x_by_row[row] = min(min_x_by_row.get(row, x), x)
    # min_offset -> max_offset across rows 0..2 spans 1.0mm total.
    assert abs((min_x_by_row[2] - min_x_by_row[0]) - 1.0) < 1e-6


# ── step_idx monotonicity ─────────────────────────────────────────


def test_material_grid_trace_records_have_increasing_step_idx():
    result = _run_grid()
    trace = result.trace
    assert trace is not None
    idx = [
        e["progress"]["step_idx"]
        for e in trace["events"]
        if e.get("progress") is not None and e["kind"] != "init"
    ]
    assert len(idx) >= 2
    for a, b in zip(idx, idx[1:]):
        assert a < b, f"step_idx not increasing: {a} >= {b}"


# ── Write trace file ──────────────────────────────────────────────


def test_material_grid_write_trace_creates_file(tmp_path):
    result = _run_grid()
    tp = tmp_path / "material.bin"
    result.write_trace(str(tp), "material_test_grid", "MaterialTestGrid")
    assert tp.exists()
    assert tp.stat().st_size > 0


def test_material_grid_write_trace_readable(tmp_path):
    from raygeo.trace import TraceFile

    result = _run_grid()
    tp = tmp_path / "material.bin"
    result.write_trace(str(tp), "material_test_grid", "MaterialTestGrid")
    trace = TraceFile(str(tp))
    assert len(trace.events) > 0
    sources = trace.sources
    assert "material_test_grid" in sources

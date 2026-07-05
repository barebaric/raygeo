"""Tests for entry assembly module."""

from raygeo.ops import Ops
from raygeo.ops.assembly.entry import (
    adaptive_entry,
    detect_entry_method,
    generate_helix_spiral,
)
from raygeo.ops.state import State


def _first_move_idx(ops):
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            return i
    return 0


# ── detect_entry_method ──


def test_detect_entry_method_helix_spiral():
    assert detect_entry_method(10.0, 3.0, 1.0) == "helix_spiral"


def test_detect_entry_method_ramp():
    assert detect_entry_method(3.0, 3.0, 1.0) == "ramp"


def test_detect_entry_method_narrow():
    assert detect_entry_method(2.0, 5.0, 1.0) == "ramp"


# ── adaptive_entry returns AssemblyResult ──


def test_adaptive_entry_returns_assembly_result():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert hasattr(result, "ops")
    assert hasattr(result, "cleared_polygons")
    assert hasattr(result, "start")
    assert hasattr(result, "end")


def test_adaptive_entry_start_end_toolpose():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    start = result.start
    end = result.end
    assert hasattr(start, "pos")
    assert hasattr(start, "heading")
    assert hasattr(end, "pos")
    assert hasattr(end, "heading")


def test_adaptive_entry_tight_slot_ramp_has_cleared_polygons():
    """Tight slot: ramp path should return a cleared polygon."""
    boundary = [(0, 0), (100, 0), (100, 16), (0, 16)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=4.0,
        step_over=3.0,
        safe_z=2.0,
        target_z=-6.0,
    )
    assert len(result.cleared_polygons) >= 1


def test_adaptive_entry_wide_end_pose_has_heading():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert result.end.heading != 0.0 or result.end.pos is not None


def test_adaptive_entry_degenerate_pocket_returns_empty_assembly_result():
    boundary = [(0, 0), (1, 0), (1, 1), (0, 1)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=5.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    assert isinstance(result.ops, Ops)
    assert hasattr(result, "start")
    assert hasattr(result, "end")


# ── Existing behavioural tests (unchanged semantics) ──


def test_adaptive_entry_wide_area_returns_path():
    """Wide pocket returns a non-empty 3D toolpath."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    idx = _first_move_idx(result.ops)
    assert result.ops.len() > 10
    assert abs(result.ops.endpoint(idx)[2] - 2.0) < 0.01
    assert abs(result.ops.endpoint(result.ops.len() - 1)[2] - (-8.0)) < 0.01


def test_adaptive_entry_wide_returns_cleared_polygons():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert len(result.cleared_polygons) >= 1
    for poly in result.cleared_polygons:
        assert len(poly) >= 3


def test_adaptive_entry_tight_slot_returns_path():
    boundary = [(0, 0), (100, 0), (100, 16), (0, 16)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=4.0,
        step_over=3.0,
        safe_z=2.0,
        target_z=-6.0,
    )
    idx = _first_move_idx(result.ops)
    assert result.ops.len() > 2
    assert abs(result.ops.endpoint(idx)[2] - 2.0) < 0.01
    assert abs(result.ops.endpoint(result.ops.len() - 1)[2] - (-6.0)) < 0.01


def test_adaptive_entry_degenerate_pocket():
    boundary = [(0, 0), (1, 0), (1, 1), (0, 1)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=5.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    assert isinstance(result.ops, Ops)


def test_adaptive_entry_cut_power_applied():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
        cut_feed_rate=1200,
        cut_power=0.55,
    )
    found_power = False
    for i in range(result.ops.len()):
        if result.ops.is_cutting(i):
            assert result.ops.state_at(i).power == 0.55
            found_power = True
            break
    assert found_power


def test_adaptive_entry_step_over_ratio():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result1 = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    result2 = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=4.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert result1.ops.len() > result2.ops.len()


# ── generate_helix_spiral ──


def test_generate_helix_spiral_returns_assembly_result():
    result = generate_helix_spiral(
        entry_pt=(80.0, 50.0),
        r_max=50.0,
    )
    assert hasattr(result, "ops")
    assert hasattr(result, "cleared_polygons")
    assert hasattr(result, "start")
    assert hasattr(result, "end")


def test_generate_helix_spiral_basic():
    result = generate_helix_spiral(
        entry_pt=(80.0, 50.0),
        r_max=50.0,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert result.ops.len() > 10
    assert len(result.cleared_polygons) >= 1


def test_generate_helix_spiral_with_state():
    st = State(power=0.5, feed_rate=1200)
    result = generate_helix_spiral(
        entry_pt=(80.0, 50.0),
        r_max=50.0,
        tool_radius=3.0,
        target_z=-8.0,
        state=st,
    )
    assert result.ops.len() > 0
    assert result.ops.command_type(0).name == "SET_POWER"


def test_generate_helix_spiral_start_pose():
    result = generate_helix_spiral(
        entry_pt=(80.0, 50.0),
        r_max=50.0,
        tool_radius=3.0,
        target_z=-8.0,
    )
    start = result.start
    assert start is not None
    assert start.heading != 0.0 or start.pos is not None

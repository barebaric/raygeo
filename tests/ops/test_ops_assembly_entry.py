"""Tests for entry assembly module."""

from raygeo.ops import Ops
from raygeo.ops.assembly.entry import adaptive_entry


def _first_move_idx(ops):
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            return i
    return 0


def test_adaptive_entry_wide_area_returns_path():
    """Wide pocket returns a non-empty 3D toolpath."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    ops, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    idx = _first_move_idx(ops)
    assert ops.len() > 10
    assert abs(ops.endpoint(idx)[2] - 2.0) < 0.01
    assert abs(ops.endpoint(ops.len() - 1)[2] - (-8.0)) < 0.01


def test_adaptive_entry_wide_returns_cleared_polygons():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    _, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert len(cleared) >= 1
    for poly in cleared:
        assert len(poly) >= 3


def test_adaptive_entry_tight_slot_returns_path():
    boundary = [(0, 0), (100, 0), (100, 16), (0, 16)]
    ops, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=4.0,
        step_over=3.0,
        safe_z=2.0,
        target_z=-6.0,
    )
    idx = _first_move_idx(ops)
    assert ops.len() > 2
    assert abs(ops.endpoint(idx)[2] - 2.0) < 0.01
    assert abs(ops.endpoint(ops.len() - 1)[2] - (-6.0)) < 0.01


def test_adaptive_entry_degenerate_pocket():
    boundary = [(0, 0), (1, 0), (1, 1), (0, 1)]
    ops, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=5.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    assert isinstance(ops, Ops)


def test_adaptive_entry_cut_power_applied():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    ops, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
        cut_feed_rate=1200,
        cut_power=0.55,
    )
    found_power = False
    for i in range(ops.len()):
        if ops.is_cutting(i):
            assert ops.state_at(i).power == 0.55
            found_power = True
            break
    assert found_power


def test_adaptive_entry_step_over_ratio():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    ops1, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ops2, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=4.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert ops1.len() > ops2.len()

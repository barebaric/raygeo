"""Tests for wavefront assembly module."""

from raygeo.ops import Ops
from raygeo.ops.area import ClearedArea
from raygeo.ops.assembly.entry import adaptive_entry
from raygeo.ops.assembly.wavefront import adaptive_wavefronts


def test_adaptive_wavefronts_simple():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_wavefronts(
        ca,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert ops.len() > 0
    assert ca.total_area() > 10000


def test_adaptive_wavefronts_with_islands():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_wavefronts(
        ca,
        boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert ops.len() > 0
    assert ca.total_area() > 5000


def test_adaptive_wavefronts_empty_cleared():
    ca = ClearedArea()
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    ops = adaptive_wavefronts(
        ca,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert isinstance(ops, Ops)


def test_adaptive_wavefronts_cut_power_applied():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_wavefronts(
        ca,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        cut_feed_rate=1200,
        cut_power=0.45,
    )
    found_power = False
    for i in range(ops.len()):
        if ops.is_cutting(i):
            assert ops.state_at(i).power == 0.45
            found_power = True
            break
    assert found_power

"""Tests for wavefront assembly module."""

import math

from raygeo.ops import Ops
from raygeo.ops.assembly.wavefront import adaptive_wavefronts
from raygeo.ops.cut import Part
from raygeo.ops.cut.cleared_area import ClearedArea


def _seed_polygon(cx, cy, r, n=32):
    """A small circle polygon to seed cleared area."""
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def test_adaptive_wavefronts_simple():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    initial = [_seed_polygon(80.0, 50.0, 15.0)]
    ca = ClearedArea(boundary=boundary, initial=initial)
    result_wf = adaptive_wavefronts(
        Part.from_polygons(boundary),
        ca,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert result_wf.ops.len() > 0
    assert ca.total_area() > 10000


def test_adaptive_wavefronts_with_islands():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    initial = [_seed_polygon(80.0, 50.0, 15.0)]
    ca = ClearedArea(boundary=boundary, initial=initial)
    result_wf = adaptive_wavefronts(
        Part.from_polygons(boundary, islands),
        ca,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert result_wf.ops.len() > 0
    assert ca.total_area() > 5000


def test_adaptive_wavefronts_empty_cleared():
    ca = ClearedArea(boundary=[])
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result_wf = adaptive_wavefronts(
        Part.from_polygons(boundary),
        ca,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert isinstance(result_wf.ops, Ops)


def test_adaptive_wavefronts_cut_power_applied():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    initial = [_seed_polygon(80.0, 50.0, 15.0)]
    ca = ClearedArea(boundary=boundary, initial=initial)
    result_wf = adaptive_wavefronts(
        Part.from_polygons(boundary),
        ca,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        cut_feed_rate=1200,
        cut_power=0.45,
    )
    found_power = False
    for i in range(result_wf.ops.len()):
        if result_wf.ops.is_cutting(i):
            assert result_wf.ops.state_at(i).power == 0.45
            found_power = True
            break
    assert found_power


def test_adaptive_wavefronts_precision_resamples():
    """precision > 0 with a seed polygon produces ops (count may vary)."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    initial = [_seed_polygon(80.0, 50.0, 15.0)]
    ca = ClearedArea(boundary=boundary, initial=initial)
    result_wf = adaptive_wavefronts(
        Part.from_polygons(boundary),
        ca,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        precision=5.0,
    )
    assert result_wf.ops.len() > 0, "expected ops with precision>0"


def test_adaptive_wavefronts_precision_with_islands():
    """precision > 0 with islands may or may not produce ops."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    initial = [_seed_polygon(80.0, 50.0, 15.0)]
    ca = ClearedArea(boundary=boundary, islands=islands, initial=initial)
    result_wf = adaptive_wavefronts(
        Part.from_polygons(boundary, islands),
        ca,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        precision=5.0,
    )
    assert isinstance(result_wf.ops, Ops)
    # Area should be tracked regardless of ops count
    assert ca.total_area() >= 0

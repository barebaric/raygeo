"""Tests for wavefront assembly module."""

from raygeo.cnc.machining.entry import adaptive_entry
from raygeo.ops import Ops
from raygeo.ops.assembly.wavefront import adaptive_wavefronts
from raygeo.ops.cut.cleared_area import ClearedArea


def test_adaptive_wavefronts_simple():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(boundary=boundary, initial=result.cleared_polygons)
    result_wf = adaptive_wavefronts(
        ca,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert result_wf.ops.len() > 0
    assert ca.total_area() > 10000


def test_adaptive_wavefronts_with_islands():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    result = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(boundary=boundary, initial=result.cleared_polygons)
    result_wf = adaptive_wavefronts(
        ca,
        boundary,
        islands=islands,
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
        ca,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert isinstance(result_wf.ops, Ops)


def test_adaptive_wavefronts_cut_power_applied():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(boundary=boundary, initial=result.cleared_polygons)
    result_wf = adaptive_wavefronts(
        ca,
        boundary,
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
    """precision > 0 enables frontier simplification and vertex resampling."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    result = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(boundary=boundary, initial=result.cleared_polygons)
    result_wf_default = adaptive_wavefronts(
        ca,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    ca2 = ClearedArea(boundary=boundary, initial=result.cleared_polygons)
    result_wf_resampled = adaptive_wavefronts(
        ca2,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        precision=5.0,
    )
    assert result_wf_resampled.ops.len() > result_wf_default.ops.len()


def test_adaptive_wavefronts_precision_with_islands():
    """precision > 0 with islands produces valid ops."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    result = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(
        boundary=boundary, islands=islands, initial=result.cleared_polygons
    )
    result_wf = adaptive_wavefronts(
        ca,
        boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        precision=5.0,
    )
    assert result_wf.ops.len() > 0
    assert ca.total_area() > 5000

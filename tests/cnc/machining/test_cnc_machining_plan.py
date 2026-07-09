"""Tests for execute_workplan in cnc/machining/plan."""

from raygeo.cnc.machining.plan import execute_workplan
from raygeo.cnc.machining.wavefront import build_wavefront_workplan
from raygeo.geo.shape.polygon import get_polygon_signed_area


def _rect(x0, y0, w, h):
    """CCW rectangle starting at (x0, y0)."""
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _area(polygons):
    return sum(abs(get_polygon_signed_area(p)) for p in polygons)


def test_execute_wavefront_workplan_runs():
    """build + execute yields a non-empty toolpath and cleared area."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        area_tolerance=1.0,
    )
    result = execute_workplan(steps, boundary)
    assert result.ops.len() > 0
    assert len(result.cleared_polygons) >= 1


def test_execute_workplan_empty_steps():
    """An empty step list yields an empty toolpath."""
    boundary = _rect(-20, -20, 40, 40)
    result = execute_workplan([], boundary)
    assert result.ops.len() == 0


def test_execute_workplan_seed_only():
    """Executing only the FlatSpiral step yields the seed disk."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
    )
    seed_steps = [s for s in steps if s["kind"] == "FlatSpiral"]
    result = execute_workplan(seed_steps, boundary)
    assert result.ops.len() > 0
    assert len(result.cleared_polygons) >= 1


def test_execute_workplan_wavefront_grows_cleared_area():
    """The full workplan clears materially more than the seed alone."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        area_tolerance=1.0,
    )
    seed = execute_workplan(
        [s for s in steps if s["kind"] == "FlatSpiral"], boundary
    )
    full = execute_workplan(steps, boundary)
    assert _area(full.cleared_polygons) > _area(seed.cleared_polygons) * 1.2


def test_execute_workplan_dict_round_trip():
    """Steps produced by the builder (dicts) are consumed unchanged by
    the executor — the dict is the build/execute contract."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary, tool_radius=3.0, step_over=2.0, target_z=-5.0
    )
    # Mutate a field to prove the executor reads the dicts, not a cache.
    steps[0]["start_angle"] = 1.234
    result = execute_workplan(steps, boundary)
    assert result.ops.len() > 0


def test_execute_workplan_unknown_kind_raises():
    """An unknown step kind is rejected with a ValueError."""
    import pytest

    boundary = _rect(-20, -20, 40, 40)
    with pytest.raises(ValueError):
        execute_workplan([{"kind": "Bogus"}], boundary)

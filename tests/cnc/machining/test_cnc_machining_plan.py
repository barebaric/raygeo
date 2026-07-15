"""Tests for Workplan struct in cnc/machining/plan."""

from raygeo.cnc.machining.entry import build_entry_workplan
from raygeo.cnc.machining.plan import Workplan
from raygeo.ops.feature.region import find_regions
from raygeo.ops.types import CommandType


def _rect(x0, y0, w, h):
    """CCW rectangle starting at (x0, y0)."""
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _build_entry(boundary, islands=None, **kwargs):
    tool_radius = kwargs.get("tool_radius", 3.0)
    regions = find_regions(
        boundary=boundary,
        islands=islands or [],
        tool_radius=tool_radius,
    )
    if not regions:
        return []
    poly, _area, entry_pt, r_max = regions[0]
    return build_entry_workplan(
        poly,
        entry_pt,
        r_max,
        islands=islands or [],
        **kwargs,
    )


def test_execute_workplan_empty_steps():
    """An empty step list yields an empty toolpath."""
    boundary = _rect(-20, -20, 40, 40)
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend([])
    result = wp.execute()
    assert result.ops.len() == 0


def test_execute_workplan_unknown_kind_raises():
    """An unknown step kind is rejected with a ValueError."""
    import pytest

    boundary = _rect(-20, -20, 40, 40)
    wp = Workplan(boundary, safe_z=2.0)
    with pytest.raises(ValueError):
        wp.extend([{"kind": "Bogus"}])


def test_workplan_rectangle_produces_cuts():
    """Execute entry workplan for 40x40 rect — ops non-empty."""
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(steps)
    result = wp.execute()
    assert result.ops.len() > 0
    assert result.ops.cut_distance() > 0


def _dumbbell():
    """Two 30x30 lobes connected by a 20x5 corridor.

    Left lobe: x=0..30, y=0..30
    Right lobe: x=50..80, y=0..30
    Corridor: x=30..50, y=12.5..17.5
    """
    return [
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 12.5),
        (50.0, 12.5),
        (50.0, 0.0),
        (80.0, 0.0),
        (80.0, 30.0),
        (50.0, 30.0),
        (50.0, 17.5),
        (30.0, 17.5),
        (30.0, 30.0),
        (0.0, 30.0),
    ]


def test_workplan_dumbbell_safe_z_between_lobes():
    """Dumbbell entry workplan — travel between lobes is at safe_z."""
    SAFE_Z = 2.0
    boundary = _dumbbell()
    steps = _build_entry(
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=SAFE_Z,
        target_z=-5.0,
    )
    wp = Workplan(boundary, safe_z=SAFE_Z)
    wp.extend(steps)
    result = wp.execute()
    ops = result.ops
    safe_z_seen = False
    for i in range(ops.len()):
        if ops.command_type(i) == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if abs(ep[2] - SAFE_Z) < 1e-6:
                safe_z_seen = True
                break
    assert safe_z_seen, f"expected a travel move at Z={SAFE_Z} between lobes"

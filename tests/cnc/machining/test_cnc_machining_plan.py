"""Tests for Workplan struct in cnc/machining/plan."""

from raygeo.cnc.machining.entry import build_entry_workplan
from raygeo.cnc.machining.plan import Workplan
from raygeo.cnc.machining.wavefront import build_wavefront_workplan
from raygeo.geo.shape.polygon import get_polygon_signed_area
from raygeo.ops.types import CommandType


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
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(steps)
    result = wp.execute()
    assert result.ops.len() > 0
    assert len(result.cleared_polygons) >= 1


def test_execute_workplan_empty_steps():
    """An empty step list yields an empty toolpath."""
    boundary = _rect(-20, -20, 40, 40)
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend([])
    result = wp.execute()
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
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(seed_steps)
    result = wp.execute()
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
    seed_steps = [s for s in steps if s["kind"] == "FlatSpiral"]
    wp_seed = Workplan(boundary, safe_z=2.0)
    wp_seed.extend(seed_steps)
    seed = wp_seed.execute()

    wp_full = Workplan(boundary, safe_z=2.0)
    wp_full.extend(steps)
    full = wp_full.execute()
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
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(steps)
    result = wp.execute()
    assert result.ops.len() > 0


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
    steps = build_entry_workplan(
        pocket_boundary=boundary,
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
    steps = build_entry_workplan(
        pocket_boundary=boundary,
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


def test_workplan_travel_uses_rapid_rate():
    """Multi-step workplan — travel moves use rapid_feed_rate."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        area_tolerance=1.0,
    )
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(steps)
    CUT_FEED = 1200
    RAPID_FEED = 5000
    result = wp.execute(cut_feed_rate=CUT_FEED, rapid_feed_rate=RAPID_FEED)
    ops = result.ops
    found_cut_feed = False
    found_travel_feed = False
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.SET_FEED_RATE:
            st = ops.state_at(i)
            fr = st.feed_rate if st is not None else None
            if fr == CUT_FEED:
                found_cut_feed = True
            if fr == RAPID_FEED:
                found_travel_feed = True
    assert found_cut_feed, f"expected SetFeedRate({CUT_FEED}) for cutting"
    assert found_travel_feed, f"expected SetFeedRate({RAPID_FEED}) for travel"


def test_workplan_determinism():
    """Same workplan executed twice yields identical Ops."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        area_tolerance=1.0,
    )
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(steps)
    r1 = wp.execute()
    r2 = wp.execute()
    assert r1.ops.len() == r2.ops.len()
    for i in range(r1.ops.len()):
        assert r1.ops.command_type(i) == r2.ops.command_type(i)
        ct = r1.ops.command_type(i)
        if ct in (CommandType.MOVE_TO, CommandType.LINE_TO):
            assert r1.ops.endpoint(i) == r2.ops.endpoint(i)

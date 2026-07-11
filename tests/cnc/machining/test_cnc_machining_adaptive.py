"""Tests for build_clearing_workplan in cnc/machining/adaptive."""

from raygeo.cnc.machining.adaptive import build_clearing_workplan
from raygeo.cnc.machining.plan import Workplan


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _dumbbell():
    """Two 30x30 lobes connected by a 20x5 corridor."""
    return [
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 12.5),
        (50.0, 12.5),
        (50.0, 17.5),
        (30.0, 17.5),
        (30.0, 30.0),
        (0.0, 30.0),
    ]


def test_build_clearing_workplan_returns_steps():
    boundary = _rect(-20, -20, 40, 40)
    steps = build_clearing_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
    )
    assert len(steps) > 0
    kinds = [s["kind"] for s in steps]
    assert "AdaptiveClear" in kinds


def test_clearing_workplan_has_entry_for_wide_pocket():
    boundary = _rect(-20, -20, 40, 40)
    steps = build_clearing_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
    )
    kinds = [s["kind"] for s in steps]
    assert any(
        k in kinds
        for k in ["HelixPlunge", "ToroidalClear", "RampEntry", "FlatSpiral"]
    )


def test_clearing_workplan_with_finishing():
    boundary = _rect(-20, -20, 40, 40)
    steps = build_clearing_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        finishing=True,
    )
    kinds = [s["kind"] for s in steps]
    assert "ProfileInner" in kinds


def test_clearing_workplan_without_finishing():
    boundary = _rect(-20, -20, 40, 40)
    steps = build_clearing_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        finishing=False,
    )
    kinds = [s["kind"] for s in steps]
    assert "ProfileInner" not in kinds


def test_clearing_workplan_no_retract():
    """No trailing Retract — the executor handles the final lift."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_clearing_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
    )
    kinds = [s["kind"] for s in steps]
    assert "Retract" not in kinds


def test_clearing_workplan_executes():
    """The produced workplan can be executed and produces ops."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_clearing_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
    )
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(steps)
    result = wp.execute()
    assert result.ops.len() > 0


def test_clearing_workplan_dumbbell():
    """Dumbbell shape with a narrow corridor."""
    boundary = _dumbbell()
    steps = build_clearing_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
    )
    kinds = [s["kind"] for s in steps]
    assert "AdaptiveClear" in kinds

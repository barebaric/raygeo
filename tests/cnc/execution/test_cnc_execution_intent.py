"""Tests for the Intent API (cnc.execution.intent)."""

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.clearing import plan_clearing
from raygeo.ops import Ops
from raygeo.ops.part import Part


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _plan(boundary, **kwargs):
    part = Part.from_polygons(boundary, [], (0.0, 0.0))
    return part, plan_clearing(part, "", **kwargs)


def test_create_intent_basic():
    """Plan -> Intent produces at least 2 nodes (compute + aggregate)."""
    boundary = _rect(-10, -10, 20, 20)
    part, plan = _plan(boundary, tool_radius=3.0, safe_z=2.0, target_z=-5.0)
    assert plan.step_count >= 1

    intent = create_intent(plan, part, 0)
    assert intent.step_count >= 1
    assert "Intent" in repr(intent)


def test_run_intent_returns_ops():
    """run_intent returns the final aggregated Ops."""
    boundary = _rect(-10, -10, 20, 20)
    part, plan = _plan(boundary, tool_radius=3.0, safe_z=2.0, target_z=-5.0)
    intent = create_intent(plan, part, 0)
    ops = run_intent(intent)
    assert isinstance(ops, Ops)
    assert ops.len() > 0


def test_run_intent_with_callback():
    """run_intent invokes the callback for each completed node."""
    boundary = _rect(-10, -10, 20, 20)
    part, plan = _plan(boundary, tool_radius=3.0, safe_z=2.0, target_z=-5.0)
    intent = create_intent(plan, part, 0)
    completed = []
    ops = run_intent(intent, on_completed=completed.append)
    assert isinstance(ops, Ops)
    # At least 2 completions: compute nodes + final aggregate
    assert len(completed) >= 2, (
        f"expected >=2 completions, got {len(completed)}"
    )

"""Tests for toroid assembly module."""

from raygeo.ops.assembly.toroid import generate_toroid
from raygeo.ops.state import State


def test_generate_toroid_basic():
    carrier = [(0.0, 0.0), (80.0, 0.0)]
    result = generate_toroid(
        carrier=carrier,
        tool_radius=3.0,
        step_distance=2.0,
        z=-5.0,
    )
    assert result.ops.len() > 0


def test_generate_toroid_returns_assembly_result():
    carrier = [(0.0, 0.0), (50.0, 0.0)]
    result = generate_toroid(
        carrier=carrier,
        tool_radius=3.0,
        step_distance=2.0,
        z=-5.0,
    )
    assert hasattr(result, "ops")
    assert hasattr(result, "cleared_polygons")
    assert hasattr(result, "start")
    assert hasattr(result, "end")


def test_generate_toroid_with_state():
    carrier = [(0.0, 0.0), (60.0, 0.0)]
    st = State(power=0.5, feed_rate=1200)
    result = generate_toroid(
        carrier=carrier,
        tool_radius=3.0,
        step_distance=2.0,
        z=-5.0,
        state=st,
    )
    assert result.ops.len() > 0
    assert result.ops.command_type(0).name == "SET_POWER"


def test_generate_toroid_ccw():
    carrier = [(0.0, 0.0), (60.0, 0.0)]
    result = generate_toroid(
        carrier=carrier,
        tool_radius=3.0,
        step_distance=2.0,
        z=-5.0,
        direction="CCW",
    )
    assert result.ops.len() > 0

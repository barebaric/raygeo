"""Tests for helix assembly module."""

import math

from raygeo.ops.assembly.helix import generate_helix
from raygeo.ops.part import Part
from raygeo.ops.state import State


def test_generate_helix_basic():
    """Basic helix produces ops with at least some commands."""
    part = Part.from_polygons([])
    result = generate_helix(
        part,
        center=(0.0, 0.0),
        start_radius=5.0,
        z_start=2.0,
        z_end=-8.0,
        pitch=2.0,
        direction="CW",
        angular_step=0.1,
    )
    assert result.ops.len() > 0
    assert part.cleared.total_area() > 0


def test_generate_helix_returns_assembly_result():
    result = generate_helix(
        Part.from_polygons([]),
        center=(10.0, 20.0),
        start_radius=3.0,
        z_start=1.0,
        z_end=-5.0,
        pitch=1.5,
    )
    assert hasattr(result, "ops")
    assert hasattr(result, "start")
    assert hasattr(result, "end")


def test_generate_helix_start_end_poses():
    result = generate_helix(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        start_radius=4.0,
        z_start=2.0,
        z_end=-6.0,
        pitch=2.0,
    )
    start = result.start
    end = result.end
    # Start pos should be on the helix circle
    d_start = math.sqrt(start.pos[0] ** 2 + start.pos[1] ** 2)
    assert abs(d_start - 4.0) < 0.01
    # End pos should be on the helix circle
    d_end = math.sqrt(end.pos[0] ** 2 + end.pos[1] ** 2)
    assert abs(d_end - 4.0) < 0.01


def test_generate_helix_ccw():
    result = generate_helix(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        start_radius=5.0,
        z_start=2.0,
        z_end=-10.0,
        pitch=2.0,
        direction="CCW",
    )
    assert result.ops.len() > 0


def test_generate_helix_with_state():
    st = State(power=0.5, feed_rate=1200)
    result = generate_helix(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        start_radius=5.0,
        z_start=2.0,
        z_end=-8.0,
        pitch=2.0,
        state=st,
    )
    assert result.ops.len() > 2
    assert result.ops.command_type(0).name == "SET_POWER"


def test_generate_helix_zero_descent():
    """No descent (z_start <= z_end) returns empty ops?"""
    result = generate_helix(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        start_radius=5.0,
        z_start=2.0,
        z_end=2.0,  # no descent
        pitch=2.0,
    )
    # Should still return a valid AssemblyResult, possibly with empty ops
    assert hasattr(result, "ops")

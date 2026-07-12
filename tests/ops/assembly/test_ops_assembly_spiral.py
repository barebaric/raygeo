"""Tests for spiral assembly module."""

from raygeo.ops.assembly.spiral import generate_spiral
from raygeo.ops.part import Part
from raygeo.ops.state import State


def test_generate_spiral_basic():
    part = Part.from_polygons([])
    result = generate_spiral(
        part,
        center=(0.0, 0.0),
        z=-5.0,
        start_radius=3.0,
        end_radius=20.0,
        revolutions=3.0,
        direction="CW",
        angular_step=0.1,
    )
    assert result.ops.len() > 0
    assert part.cleared.total_area() > 0


def test_generate_spiral_returns_assembly_result():
    result = generate_spiral(
        Part.from_polygons([]),
        center=(10.0, 20.0),
        z=-5.0,
        start_radius=3.0,
        end_radius=15.0,
        revolutions=2.0,
    )
    assert hasattr(result, "ops")
    assert hasattr(result, "start")
    assert hasattr(result, "end")


def test_generate_spiral_start_end_poses():
    result = generate_spiral(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        z=-5.0,
        start_radius=4.0,
        end_radius=20.0,
        revolutions=2.0,
    )
    assert result.start is not None
    assert result.end is not None


def test_generate_spiral_with_state():
    st = State(power=0.5, feed_rate=1200)
    result = generate_spiral(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        z=-5.0,
        start_radius=3.0,
        end_radius=20.0,
        revolutions=3.0,
        state=st,
    )
    assert result.ops.len() > 0
    assert result.ops.command_type(0).name == "SET_POWER"


def test_generate_spiral_ccw():
    result = generate_spiral(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        z=-5.0,
        start_radius=3.0,
        end_radius=20.0,
        revolutions=3.0,
        direction="CCW",
    )
    assert result.ops.len() > 0

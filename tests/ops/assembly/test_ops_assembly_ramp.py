"""Tests for ramp assembly module."""

from raygeo.ops.assembly.ramp import generate_ramp
from raygeo.ops.cut import Part
from raygeo.ops.state import State


def test_generate_ramp_basic():
    result = generate_ramp(
        Part.from_polygons([]),
        start=(0.0, 0.0),
        end=(100.0, 0.0),
        z_start=2.0,
        z_end=-6.0,
    )
    assert result.ops.len() > 0
    assert len(result.cleared_polygons) >= 1


def test_generate_ramp_returns_assembly_result():
    result = generate_ramp(
        Part.from_polygons([]),
        start=(0.0, 0.0),
        end=(50.0, 0.0),
        z_start=2.0,
        z_end=-5.0,
    )
    assert hasattr(result, "ops")
    assert hasattr(result, "cleared_polygons")
    assert hasattr(result, "start")
    assert hasattr(result, "end")


def test_generate_ramp_start_end_poses():
    result = generate_ramp(
        Part.from_polygons([]),
        start=(10.0, 10.0),
        end=(90.0, 10.0),
        z_start=2.0,
        z_end=-8.0,
    )
    assert result.start is not None
    assert result.end is not None


def test_generate_ramp_with_state():
    st = State(power=0.5, feed_rate=1200)
    result = generate_ramp(
        Part.from_polygons([]),
        start=(0.0, 0.0),
        end=(100.0, 0.0),
        z_start=2.0,
        z_end=-6.0,
        state=st,
    )
    assert result.ops.len() > 0
    assert result.ops.command_type(0).name == "SET_POWER"


def test_generate_ramp_linear():
    result = generate_ramp(
        Part.from_polygons([]),
        start=(0.0, 0.0),
        end=(100.0, 0.0),
        z_start=2.0,
        z_end=-6.0,
        style="linear",
    )
    assert result.ops.len() > 0

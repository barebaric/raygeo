import pytest

from raygeo.ops import Ops
from raygeo.ops.types import CommandCategory, CommandType


def test_linearize_all():
    ops = Ops()
    ops.move_to(10, 0)
    ops.line_to(20, 0)
    ops.arc_to(10, 10, i=-10, j=0, clockwise=False)  # Semicircle
    ops.set_power(1.0)

    ops.linearize_all()

    assert ops.len() > 4  # Move, Line, SetPower, plus linearized arc
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.LINE_TO
    # All geometric commands after the first two should be LineTo
    moving_cmds_after = [
        i
        for i in range(2, ops.len())
        if ops.category(i) == CommandCategory.MOVING
    ]
    assert all(
        ops.command_type(i) == CommandType.LINE_TO for i in moving_cmds_after
    )
    # Check that state command is still there
    assert any(
        ops.command_type(i) == CommandType.SET_POWER for i in range(ops.len())
    )


def test_linearize_curves():
    """Tests that linearize_curves replaces beziers with lines."""
    ops = Ops()
    ops.move_to(0, 0, 0)
    ops.bezier_to(control1=(10, 0, 0), control2=(10, 10, 0), end=(0, 10, 0))
    ops.set_power(0.8)
    ops.line_to(5, 5, 0)

    assert ops.command_type(1) == CommandType.BEZIER_TO
    ops.linearize_curves()

    assert not any(
        ops.command_type(i) == CommandType.BEZIER_TO
        or ops.command_type(i) == CommandType.QUADRATIC_BEZIER_TO
        for i in range(ops.len())
    )
    assert ops.command_type(0) == CommandType.MOVE_TO
    moving_indices = [
        i
        for i in range(1, ops.len() - 1)
        if ops.category(i) == CommandCategory.MOVING
    ]
    assert all(
        ops.command_type(i) == CommandType.LINE_TO for i in moving_indices
    )
    assert ops.command_type(ops.len() - 2) == CommandType.SET_POWER
    assert ops.command_type(ops.len() - 1) == CommandType.LINE_TO
    assert ops.endpoint(ops.len() - 1) == (5, 5, 0)


def test_linearize_curves_preserves_arcs():
    """Tests that linearize_curves does not touch arcs."""
    ops = Ops()
    ops.move_to(10, 0)
    ops.arc_to(0, 10, i=-10, j=0, clockwise=False)
    ops.bezier_to(control1=(0, 10, 0), control2=(-10, 10, 0), end=(-10, 0, 0))

    ops.linearize_curves()

    assert ops.command_type(1) == CommandType.ARC_TO
    assert ops.command_type(ops.len() - 1) == CommandType.LINE_TO
    assert not any(
        ops.command_type(i)
        in (CommandType.BEZIER_TO, CommandType.QUADRATIC_BEZIER_TO)
        for i in range(ops.len())
    )


def test_linearize_arcs():
    """Tests that linearize_arcs replaces arcs with lines."""
    ops = Ops()
    ops.move_to(10, 0)
    ops.line_to(20, 0)
    ops.arc_to(10, 10, i=-10, j=0, clockwise=False)
    ops.set_power(1.0)
    ops.move_to(10, 10)
    ops.bezier_to(control1=(10, 10, 0), control2=(20, 10, 0), end=(20, 0, 0))


def test_linearize_arcs_preserves_beziers():
    """Tests that linearize_arcs does not touch bezier curves."""
    ops = Ops()
    ops.move_to(0, 0)
    ops.bezier_to(control1=(10, 0, 0), control2=(10, 10, 0), end=(0, 10, 0))
    ops.set_power(0.8)

    ops.linearize_arcs()

    bezier_indices = [
        i
        for i in range(ops.len())
        if ops.command_type(i) == CommandType.BEZIER_TO
    ]
    assert len(bezier_indices) == 1
    info = ops.inspect(bezier_indices[0])
    assert info.control1 == (10, 0, 0)
    assert info.control2 == (10, 10, 0)
    assert info.end == (0, 10, 0)


def test_linearize_arc():
    ops = Ops()
    ops.arc_to(10, 0, 5, 0, False)
    result = ops.linearize(0, (0.0, 0.0, 0.0))
    assert result.len() > 1
    for i in range(result.len()):
        assert result.command_type(i) == CommandType.LINE_TO


def test_linearize_bezier():
    ops = Ops()
    ops.move_to(0, 0)
    ops.bezier_to((5, 0, 0), (10, 5, 0), (15, 0, 0))
    result = ops.linearize(1, (0.0, 0.0, 0.0))
    assert result.len() > 1
    for i in range(result.len()):
        assert result.command_type(i) == CommandType.LINE_TO


def test_linearize_scanline():
    ops = Ops()
    ops.scan_to(10, 0, 0, power_values=bytearray([100, 200, 100]))
    result = ops.linearize(0, (0.0, 0.0, 0.0))
    assert result.len() > 1


def test_linearize_line():
    ops = Ops()
    ops.line_to(10, 0)
    result = ops.linearize(0, (0.0, 0.0, 0.0))
    assert result.len() == 1
    assert result.command_type(0) == CommandType.LINE_TO


def test_linearize_move():
    ops = Ops()
    ops.move_to(5, 5)
    result = ops.linearize(0, (0.0, 0.0, 0.0))
    assert result.len() == 1
    assert result.command_type(0) == CommandType.MOVE_TO


def test_linearize_unsupported():
    ops = Ops()
    ops.set_power(1.0)
    with pytest.raises(TypeError):
        ops.linearize(0, (0.0, 0.0, 0.0))

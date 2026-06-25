import pytest

from raygeo.ops import Ops
from raygeo.ops.state import CoolantMode
from raygeo.ops.types import CommandCategory, CommandType


@pytest.fixture
def sample_ops():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.set_power(0.5)
    ops.set_coolant(CoolantMode.AIR)
    return ops


@pytest.fixture
def empty_ops():
    return Ops()


def test_get_frame(sample_ops):
    frame = sample_ops.get_frame(power=1.0, feed_rate=500)
    assert (
        sum(
            1
            for i in range(frame.len())
            if frame.category(i) == CommandCategory.MOVING
            and frame.command_type(i) == CommandType.MOVE_TO
        )
        == 1
    )
    assert (
        sum(
            1
            for i in range(frame.len())
            if frame.category(i) == CommandCategory.MOVING
            and frame.command_type(i) == CommandType.LINE_TO
        )
        == 4
    )

    min_x, min_y, max_x, max_y = sample_ops.rect()

    expected_points = [
        (min_x, min_y, 0.0),
        (min_x, max_y, 0.0),
        (max_x, max_y, 0.0),
        (max_x, min_y, 0.0),
        (min_x, min_y, 0.0),
    ]

    frame_points = [
        frame.endpoint(i)
        for i in range(frame.len())
        if frame.category(i) == CommandCategory.MOVING
    ]
    assert frame_points == expected_points


def test_get_frame_empty(empty_ops):
    frame = empty_ops.get_frame()
    assert len(frame) == 0

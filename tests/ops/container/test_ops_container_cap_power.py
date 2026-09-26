import pytest

from raygeo.ops import Ops
from raygeo.ops.state import PowerMode
from raygeo.ops.types import CommandType


def power_commands(ops):
    return [
        (idx, ops.power(idx))
        for idx in range(ops.len())
        if ops.command_type(idx) == CommandType.SET_POWER
    ]


@pytest.fixture
def mixed_ops():
    ops = Ops()
    ops.set_power(0.8)
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.scan_to(10, 5, 0, power_values=[255, 128, 0])
    ops.set_power_mode(PowerMode.CONSTANT)
    ops.set_feed_rate(1000)
    return ops


def test_cap_power_clamps_set_power(mixed_ops):
    mixed_ops.cap_power(0.1)
    assert power_commands(mixed_ops) == [(0, 0.1)]


def test_cap_power_keeps_lower_power():
    ops = Ops()
    ops.set_power(0.05)
    ops.cap_power(0.1)
    assert power_commands(ops) == [(0, 0.05)]


def test_cap_power_clamps_scanline_bytes(mixed_ops):
    mixed_ops.cap_power(0.1)
    assert list(mixed_ops.scanline_data(3)) == [26, 26, 0]


def test_cap_power_scanline_at_cap_boundary():
    ops = Ops()
    ops.scan_to(10, 0, 0, power_values=[25, 26, 27])
    ops.cap_power(0.1)
    assert list(ops.scanline_data(0)) == [25, 26, 26]


def test_cap_power_full_power_is_noop(mixed_ops):
    mixed_ops.cap_power(1.0)
    assert power_commands(mixed_ops) == [(0, 0.8)]
    assert list(mixed_ops.scanline_data(3)) == [255, 128, 0]


def test_cap_power_zero_disables_beam(mixed_ops):
    mixed_ops.cap_power(0.0)
    assert power_commands(mixed_ops) == [(0, 0.0)]
    assert list(mixed_ops.scanline_data(3)) == [0, 0, 0]


def test_cap_power_clamps_invalid_max_power(mixed_ops):
    mixed_ops.cap_power(2.0)
    assert power_commands(mixed_ops) == [(0, 0.8)]
    assert list(mixed_ops.scanline_data(3)) == [255, 128, 0]

    mixed_ops.cap_power(-1.0)
    assert power_commands(mixed_ops) == [(0, 0.0)]
    assert list(mixed_ops.scanline_data(3)) == [0, 0, 0]


def test_cap_power_preserves_other_commands(mixed_ops):
    mixed_ops.cap_power(0.1)
    assert mixed_ops.endpoint(1) == (0.0, 0.0, 0.0)
    assert mixed_ops.endpoint(2) == (10.0, 0.0, 0.0)
    assert mixed_ops.endpoint(3) == (10.0, 5.0, 0.0)
    assert mixed_ops.command_type(4) == CommandType.SET_POWER_MODE
    assert mixed_ops.command_type(5) == CommandType.SET_FEED_RATE


def test_cap_power_keeps_command_count(mixed_ops):
    count_before = mixed_ops.len()
    mixed_ops.cap_power(0.1)
    assert mixed_ops.len() == count_before


def test_cap_power_empty_ops():
    ops = Ops()
    ops.cap_power(0.5)
    assert ops.len() == 0


def test_cap_power_preserves_geometry(mixed_ops):
    rect_before = mixed_ops.rect()
    mixed_ops.cap_power(0.2)
    assert mixed_ops.rect() == pytest.approx(rect_before)


def test_cap_power_after_clone_only_affects_target(mixed_ops):
    clone = mixed_ops.copy()
    mixed_ops.cap_power(0.1)
    assert power_commands(mixed_ops) == [(0, 0.1)]
    assert list(clone.scanline_data(3)) == [255, 128, 0]
    assert power_commands(clone) == [(0, 0.8)]

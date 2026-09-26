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


def test_cap_power_returns_new_ops(mixed_ops):
    capped = mixed_ops.cap_power(0.1)
    assert capped is not mixed_ops
    assert capped.len() == mixed_ops.len()


def test_cap_power_original_unchanged(mixed_ops):
    mixed_ops.cap_power(0.1)
    assert power_commands(mixed_ops) == [(0, 0.8)]
    assert list(mixed_ops.scanline_data(3)) == [255, 128, 0]


def test_cap_power_clamps_set_power(mixed_ops):
    capped = mixed_ops.cap_power(0.1)
    assert power_commands(capped) == [(0, 0.1)]


def test_cap_power_keeps_lower_power():
    ops = Ops()
    ops.set_power(0.05)
    capped = ops.cap_power(0.1)
    assert power_commands(capped) == [(0, 0.05)]


def test_cap_power_clamps_scanline_bytes(mixed_ops):
    capped = mixed_ops.cap_power(0.1)
    assert list(capped.scanline_data(3)) == [26, 26, 0]


def test_cap_power_scanline_at_cap_boundary():
    ops = Ops()
    ops.scan_to(10, 0, 0, power_values=[25, 26, 27])
    capped = ops.cap_power(0.1)
    assert list(capped.scanline_data(0)) == [25, 26, 26]


def test_cap_power_full_power_is_noop(mixed_ops):
    capped = mixed_ops.cap_power(1.0)
    assert power_commands(capped) == [(0, 0.8)]
    assert list(capped.scanline_data(3)) == [255, 128, 0]


def test_cap_power_zero_disables_beam(mixed_ops):
    capped = mixed_ops.cap_power(0.0)
    assert power_commands(capped) == [(0, 0.0)]
    assert list(capped.scanline_data(3)) == [0, 0, 0]


def test_cap_power_clamps_invalid_max_power(mixed_ops):
    capped = mixed_ops.cap_power(2.0)
    assert power_commands(capped) == [(0, 0.8)]
    assert list(capped.scanline_data(3)) == [255, 128, 0]

    capped = mixed_ops.cap_power(-1.0)
    assert power_commands(capped) == [(0, 0.0)]
    assert list(capped.scanline_data(3)) == [0, 0, 0]


def test_cap_power_preserves_other_commands(mixed_ops):
    capped = mixed_ops.cap_power(0.1)
    assert capped.endpoint(1) == (0.0, 0.0, 0.0)
    assert capped.endpoint(2) == (10.0, 0.0, 0.0)
    assert capped.endpoint(3) == (10.0, 5.0, 0.0)
    assert capped.command_type(4) == CommandType.SET_POWER_MODE
    assert capped.command_type(5) == CommandType.SET_FEED_RATE


def test_cap_power_empty_ops():
    ops = Ops()
    capped = ops.cap_power(0.5)
    assert capped.len() == 0


def test_cap_power_preserves_geometry_after_transform(mixed_ops):
    capped = mixed_ops.cap_power(0.2)
    rect_before = mixed_ops.rect()
    rect_after = capped.rect()
    assert rect_before == pytest.approx(rect_after)

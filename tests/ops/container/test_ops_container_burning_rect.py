import pytest

from raygeo.ops import Ops
from raygeo.ops.types import RasterMode, SectionType

DIST = 5.0


def test_empty_ops():
    assert Ops().burning_rect() == (0.0, 0.0, 0.0, 0.0)


def test_travel_moves_excluded():
    ops = Ops()
    ops.move_to(0, 0)
    ops.move_to(10, 10)
    assert ops.burning_rect() == (0.0, 0.0, 0.0, 0.0)


def test_powered_move_included():
    ops = Ops()
    ops.move_to(0, 0)
    ops.set_power(0.5)
    ops.line_to(10, 10)
    assert ops.burning_rect() == pytest.approx((0.0, 0.0, 10.0, 10.0))


def test_default_power_is_zero():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    assert ops.burning_rect() == (0.0, 0.0, 0.0, 0.0)


def test_zero_power_move_excluded():
    ops = Ops()
    ops.move_to(0, 0)
    ops.set_power(0.5)
    ops.line_to(10, 10)
    ops.set_power(0.0)
    ops.move_to(10, 10)
    ops.line_to(20, 10)
    assert ops.burning_rect() == pytest.approx((0.0, 0.0, 10.0, 10.0))
    assert ops.rect() == pytest.approx((0.0, 0.0, 20.0, 10.0))


def test_power_restored_after_zero():
    ops = Ops()
    ops.move_to(0, 0)
    ops.set_power(0.5)
    ops.line_to(10, 10)
    ops.set_power(0.0)
    ops.move_to(10, 10)
    ops.line_to(20, 10)
    ops.set_power(0.5)
    ops.line_to(30, 10)
    assert ops.burning_rect() == pytest.approx((0.0, 0.0, 30.0, 10.0))


def test_scanline_padding_trimmed():
    ops = Ops()
    ops.move_to(0.0, 0.0)
    ops.scan_to(6.0, 0.0, 0.0, power_values=bytearray([0, 100, 200, 0]))
    assert ops.burning_rect() == pytest.approx((1.5, 0.0, 4.5, 0.0))


def test_scanline_all_zero_ignored():
    ops = Ops()
    ops.move_to(0.0, 0.0)
    ops.scan_to(6.0, 0.0, 0.0, power_values=bytearray([0, 0, 0, 0]))
    assert ops.burning_rect() == (0.0, 0.0, 0.0, 0.0)


def test_scanline_no_padding_matches_rect():
    ops = Ops()
    ops.move_to(10.0, 20.0)
    ops.scan_to(30.0, 20.0, 0.0, power_values=bytearray([50, 100, 150]))
    assert ops.burning_rect() == pytest.approx(ops.rect())


def test_frame_around_overscanned_raster():
    content = Ops()
    content.ops_section_start(
        SectionType.RASTER_FILL,
        "wp1",
        raster_mode=RasterMode.VARIABLE_POWER,
    )
    content.move_to(10, 20, 0)
    content.scan_to(30, 20, 0, power_values=bytearray([100] * 20))
    content.ops_section_end(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    overscanned = content.copy()
    overscanned.apply_overscan(DIST)

    assert overscanned.rect() == pytest.approx(
        (5.0, 20.0, 35.0, 20.0), abs=1e-6
    )
    assert overscanned.burning_rect() == pytest.approx(
        (10.0, 20.0, 30.0, 20.0), abs=1e-6
    )

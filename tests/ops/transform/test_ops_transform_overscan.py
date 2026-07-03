import math

import pytest

from raygeo.ops import Ops
from raygeo.ops.state import AirAssistMode
from raygeo.ops.types import CommandType, SectionType

DIST = 5.0


class TestBasic:
    def test_empty_ops(self):
        ops = Ops()
        ops.apply_overscan(DIST)
        assert ops.is_empty()

    def test_zero_distance_no_op(self):
        ops = Ops()
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
        orig = ops.len()
        ops.apply_overscan(0.0)
        assert ops.len() == orig

    def test_no_raster_section_no_change(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(5, 5, 0)
        orig_ep0 = ops.endpoint(0)
        orig_ep1 = ops.endpoint(1)
        ops.apply_overscan(DIST)
        assert ops.endpoint(0) == orig_ep0
        assert ops.endpoint(1) == orig_ep1


class TestConstantPower:
    def test_single_horizontal_line(self):
        ops = Ops()
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 20, 5)
        ops.line_to(30, 20, 5)
        ops.ops_section_end(SectionType.RASTER_FILL)
        ops.apply_overscan(DIST)

        assert ops.len() == 9
        assert ops.command_type(1) == CommandType.MOVE_TO
        assert ops.endpoint(1) == pytest.approx((5.0, 20.0, 5.0))
        assert ops.command_type(5) == CommandType.LINE_TO
        assert ops.endpoint(5) == pytest.approx((30.0, 20.0, 5.0))
        assert ops.command_type(7) == CommandType.LINE_TO
        assert ops.endpoint(7) == pytest.approx((35.0, 20.0, 5.0))

    def test_preserves_state_with_intermediate_power(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.set_air_assist(AirAssistMode.ON)
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 20, 0)
        ops.line_to(20, 20, 0)
        ops.move_to(30, 20, 0)
        ops.set_power(0.4)
        ops.line_to(40, 20, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
        ops.apply_overscan(DIST)

        assert ops.command_type(0) == CommandType.SET_POWER
        assert ops.power(0) == 0.8
        assert ops.command_type(1) == CommandType.SET_AIR_ASSIST
        assert ops.command_type(2) == CommandType.OPS_SECTION_START
        assert ops.command_type(17) == CommandType.OPS_SECTION_END
        assert ops.len() == 18

        assert ops.command_type(3) == CommandType.MOVE_TO
        assert ops.endpoint(3) == pytest.approx((5.0, 20.0, 0.0))
        assert ops.command_type(4) == CommandType.SET_POWER
        assert ops.power(4) == 0
        assert ops.command_type(6) == CommandType.SET_POWER
        assert ops.power(6) == 0.8
        assert ops.command_type(7) == CommandType.LINE_TO
        assert ops.endpoint(7) == pytest.approx((20.0, 20.0, 0.0))
        assert ops.command_type(8) == CommandType.SET_POWER
        assert ops.power(8) == 0
        assert ops.endpoint(9) == pytest.approx((25.0, 20.0, 0.0))

        assert ops.command_type(10) == CommandType.MOVE_TO
        assert ops.command_type(13) == CommandType.SET_POWER
        assert ops.power(13) == 0.4
        assert ops.endpoint(14) == pytest.approx((40.0, 20.0, 0.0))
        assert ops.endpoint(16) == pytest.approx((45.0, 20.0, 0.0))

    def test_multiple_bidirectional_lines(self):
        ops = Ops()
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 20, 0)
        ops.line_to(30, 20, 0)
        ops.move_to(30, 22, 0)
        ops.line_to(10, 22, 0)
        ops.move_to(5, 30, 0)
        ops.line_to(15, 40, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
        ops.apply_overscan(DIST)

        move_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.MOVE_TO
        ]
        line_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.LINE_TO
        ]

        assert len(move_indices) == 3
        assert len(line_indices) == 9

        assert ops.endpoint(move_indices[0]) == pytest.approx(
            (10 - DIST, 20, 0)
        )
        assert ops.endpoint(line_indices[2]) == pytest.approx(
            (30 + DIST, 20, 0)
        )
        assert ops.endpoint(move_indices[1]) == pytest.approx(
            (30 + DIST, 22, 0)
        )
        assert ops.endpoint(line_indices[5]) == pytest.approx(
            (10 - DIST, 22, 0)
        )

        norm_v = 1.0 / math.sqrt(2.0)
        ox = oy = DIST * norm_v
        assert ops.endpoint(move_indices[2]) == pytest.approx(
            (5 - ox, 30 - oy, 0)
        )
        assert ops.endpoint(line_indices[8]) == pytest.approx(
            (15 + ox, 40 + oy, 0)
        )

    def test_zero_length_line_unchanged(self):
        ops = Ops()
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(10, 10, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
        orig = ops.len()
        ops.apply_overscan(DIST)
        assert ops.len() == orig


class TestScanLine:
    def test_variable_power_scanline(self):
        power_vals = bytearray(range(1, 41))
        ops = Ops()
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 20, 0)
        ops.scan_to(30, 20, 0, power_values=power_vals)
        ops.ops_section_end(SectionType.RASTER_FILL)
        ops.apply_overscan(DIST)

        assert ops.len() == 4
        assert ops.command_type(1) == CommandType.MOVE_TO
        assert ops.endpoint(1) == pytest.approx((5.0, 20.0, 0.0))
        assert ops.command_type(2) == CommandType.SCAN_LINE
        assert ops.endpoint(2) == pytest.approx((35.0, 20.0, 0.0))

        num_pad = 10
        pad_bytes = bytearray([0] * num_pad)
        expected = pad_bytes + power_vals + pad_bytes
        assert ops.scanline_data(2) == expected

    def test_scanline_preserves_preceding_state(self):
        ops = Ops()
        ops.set_power(0.5)
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 20, 0)
        ops.scan_to(20, 20, 0, power_values=bytearray([100, 200]))
        ops.ops_section_end(SectionType.RASTER_FILL)
        ops.apply_overscan(DIST)

        assert ops.len() == 5
        assert ops.command_type(0) == CommandType.SET_POWER
        assert ops.power(0) == 0.5
        assert ops.command_type(1) == CommandType.OPS_SECTION_START
        assert ops.command_type(2) == CommandType.MOVE_TO
        assert ops.endpoint(2) == pytest.approx((5.0, 20.0, 0.0))
        assert ops.command_type(3) == CommandType.SCAN_LINE
        assert ops.endpoint(3) == pytest.approx((25.0, 20.0, 0.0))

        num_pad = 1
        pad_bytes = bytearray([0] * num_pad)
        expected = pad_bytes + bytearray([100, 200]) + pad_bytes
        assert ops.scanline_data(3) == expected


# ── smoke tests from original test_ops_assembly ──


def test_assembly_apply_overscan():
    ops = Ops()
    ops.ops_section_start(SectionType.RASTER_FILL, "wp")
    ops.set_power(1.0)
    ops.move_to(0, 0)
    ops.scan_to(10, 0, 0)
    ops.move_to(10, 1)
    ops.scan_to(0, 1, 0)
    ops.ops_section_end(SectionType.RASTER_FILL)
    original_len = ops.len()
    ops.apply_overscan(1.0)
    assert ops.len() >= original_len

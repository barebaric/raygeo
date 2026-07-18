from raygeo.ops import Ops
from raygeo.ops.types import CommandType


class TestBidirScanOffset:
    def test_empty_ops(self):
        ops = Ops()
        ops.apply_bidir_scan_offset(1.0)
        assert ops.is_empty()

    def test_zero_offset_noop(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.scan_to(10, 0, 0, power_values=[128] * 10)
        n = ops.len()
        ops.apply_bidir_scan_offset(0.0)
        assert ops.len() == n

    def test_ltr_unchanged(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.scan_to(10, 0, 0, power_values=[128] * 10)
        ops.apply_bidir_scan_offset(5.0)
        assert ops.len() == 2
        assert ops.endpoint(0) == (0.0, 0.0, 0.0)
        assert ops.endpoint(1) == (10.0, 0.0, 0.0)

    def test_rtl_shifted(self):
        ops = Ops()
        ops.move_to(10, 0, 0)
        ops.scan_to(0, 0, 0, power_values=[128] * 10)
        ops.apply_bidir_scan_offset(3.0)
        assert ops.len() == 2
        assert ops.endpoint(0) == (13.0, 0.0, 0.0)
        assert ops.endpoint(1) == (3.0, 0.0, 0.0)

    def test_state_between_transferred(self):
        ops = Ops()
        ops.move_to(10, 0, 0)
        ops.set_power(0.5)
        ops.set_feed_rate(500.0)
        ops.scan_to(0, 0, 0, power_values=[64] * 5)
        ops.apply_bidir_scan_offset(2.0)
        assert ops.len() == 4
        assert ops.command_type(0) == CommandType.MOVE_TO
        assert ops.command_type(1) == CommandType.SET_POWER
        assert ops.command_type(2) == CommandType.SET_FEED_RATE
        assert ops.command_type(3) == CommandType.SCAN_LINE

    def test_multiple_passes(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.scan_to(10, 0, 0, power_values=[100] * 10)
        ops.move_to(10, 1, 0)
        ops.scan_to(0, 1, 0, power_values=[200] * 10)
        ops.apply_bidir_scan_offset(4.0)
        assert ops.len() == 4
        assert ops.endpoint(0) == (0.0, 0.0, 0.0)
        assert ops.endpoint(1) == (10.0, 0.0, 0.0)
        assert ops.endpoint(2) == (14.0, 1.0, 0.0)
        assert ops.endpoint(3) == (4.0, 1.0, 0.0)

    def test_yz_preserved(self):
        ops = Ops()
        ops.move_to(10, 5, 2)
        ops.scan_to(0, 5, 2, power_values=[128] * 10)
        ops.apply_bidir_scan_offset(3.0)
        assert ops.endpoint(0) == (13.0, 5.0, 2.0)
        assert ops.endpoint(1) == (3.0, 5.0, 2.0)

    def test_power_values_preserved(self):
        ops = Ops()
        ops.move_to(10, 0, 0)
        ops.scan_to(0, 0, 0, power_values=[10, 20, 30])
        ops.apply_bidir_scan_offset(1.0)
        assert list(ops.scanline_data(1)) == [10, 20, 30]

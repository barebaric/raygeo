from raygeo.ops import Ops
from raygeo.ops.types import CommandType


class TestMultiPass:
    def test_empty_ops(self):
        ops = Ops()
        ops.apply_multipass(3, 0.0)
        assert ops.is_empty()

    def test_single_pass_noop(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        n = ops.len()
        ops.apply_multipass(1, 0.0)
        assert ops.len() == n

    def test_two_passes_no_z(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.apply_multipass(2, 0.0)
        assert ops.len() == 4
        assert ops.endpoint(0) == (0.0, 0.0, 0.0)
        assert ops.endpoint(1) == (10.0, 0.0, 0.0)
        assert ops.endpoint(2) == (0.0, 0.0, 0.0)
        assert ops.endpoint(3) == (10.0, 0.0, 0.0)

    def test_three_passes_no_z(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(5, 0, 0)
        ops.apply_multipass(3, 0.0)
        assert ops.len() == 6

    def test_z_step_down_pass2(self):
        ops = Ops()
        ops.move_to(0, 0, 10)
        ops.line_to(5, 0, 10)
        ops.apply_multipass(2, 2.5)
        # Pass 2 (i=1): z = 10 - 2.5 = 7.5
        assert ops.endpoint(0) == (0.0, 0.0, 10.0)
        assert ops.endpoint(1) == (5.0, 0.0, 10.0)
        assert ops.endpoint(2) == (0.0, 0.0, 7.5)
        assert ops.endpoint(3) == (5.0, 0.0, 7.5)

    def test_z_step_down_pass3(self):
        ops = Ops()
        ops.move_to(0, 0, 10)
        ops.line_to(5, 0, 10)
        ops.apply_multipass(3, 2.0)
        # Pass 2 (i=1): z = 10 - 2.0 = 8.0
        # Pass 3 (i=2): z = 10 - 4.0 = 6.0
        assert ops.endpoint(2) == (0.0, 0.0, 8.0)
        assert ops.endpoint(3) == (5.0, 0.0, 8.0)
        assert ops.endpoint(4) == (0.0, 0.0, 6.0)
        assert ops.endpoint(5) == (5.0, 0.0, 6.0)

    def test_power_values_preserved(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.scan_to(10, 0, 0, power_values=[50, 100, 150])
        ops.apply_multipass(2, 0.0)
        assert list(ops.scanline_data(1)) == [50, 100, 150]
        assert list(ops.scanline_data(3)) == [50, 100, 150]

    def test_state_between_transferred(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.set_power(0.7)
        ops.line_to(5, 0, 0)
        ops.apply_multipass(2, 0.0)
        # Pass 1: move_to, set_power, line_to
        # Pass 2: move_to, set_power, line_to
        assert ops.len() == 6
        assert ops.command_type(0) == CommandType.MOVE_TO
        assert ops.command_type(1) == CommandType.SET_POWER
        assert ops.command_type(2) == CommandType.LINE_TO
        assert ops.command_type(3) == CommandType.MOVE_TO
        assert ops.command_type(4) == CommandType.SET_POWER
        assert ops.command_type(5) == CommandType.LINE_TO

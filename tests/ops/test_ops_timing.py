"""
Tests for timing estimation functionality.
"""

from raygeo.ops import Ops
from raygeo.ops.types import CommandCategory


class TestTiming:
    """Test cases for timing estimation."""

    def test_empty_commands(self):
        """Test that empty command list returns 0 time."""
        ops = Ops()
        assert ops.estimate_time() == 0.0

    def test_single_move_command(self):
        """Test timing estimation for a single move command."""
        ops = Ops()
        ops.move_to(10, 10, 0)
        # MoveToCommand is not a cutting command, so it should use travel speed
        # Distance = sqrt(10^2 + 10^2) = 14.14mm
        # At 3000mm/min = 50mm/s, time = 14.14/50 = 0.283s + acceleration
        actual_time = ops.estimate_time()
        # Should be around 0.33s with acceleration
        assert 0.3 < actual_time < 0.4

    def test_single_line_command(self):
        """Test timing estimation for a single line command."""
        ops = Ops()
        ops.line_to(10, 0, 0)
        # LineToCommand is a cutting command, so it should use cut speed
        # Distance = 10mm
        # At 1000mm/min = 16.67mm/s, time = 10/16.67 = 0.6s + acceleration
        actual_time = ops.estimate_time()
        # Should be around 0.62s with acceleration
        assert 0.6 < actual_time < 0.65

    def test_custom_speeds(self):
        """Test timing estimation with custom speeds."""
        ops = Ops()
        ops.line_to(60, 0, 0)
        # Distance = 60mm
        # At 1200mm/min = 20mm/s, time = 60/20 = 3s + acceleration
        actual_time = ops.estimate_time(default_cut_speed=1200.0)
        # Should be around 3.02s with acceleration
        assert 3.0 < actual_time < 3.05

    def test_speed_commands(self):
        """Test timing estimation with speed change commands."""
        ops = Ops()
        ops.set_cut_speed(600)  # 10mm/s
        ops.line_to(50, 0, 0)  # 5s at 10mm/s
        ops.set_travel_speed(1200)  # 20mm/s
        ops.move_to(50, 50, 0)  # 2.5s at 20mm/s
        actual_time = ops.estimate_time()
        # Should be around 7.53s with acceleration
        assert 7.5 < actual_time < 7.55

    def test_acceleration_disabled(self):
        """Test timing estimation with acceleration disabled."""
        ops = Ops()
        ops.line_to(10, 0, 0)
        # With acceleration=0, should use simple distance/speed calculation
        actual_time = ops.estimate_time(acceleration=0)
        expected_time = 0.6  # 10mm / (1000mm/min / 60) = 0.6s
        assert abs(actual_time - expected_time) < 0.01

    def test_acceleration_enabled(self):
        """Test timing estimation with acceleration enabled."""
        ops = Ops()
        ops.line_to(10, 0, 0)
        # With acceleration, should be slightly longer due to acceleration
        time_with_accel = ops.estimate_time(acceleration=1000.0)
        time_without_accel = ops.estimate_time(acceleration=0.0)
        assert time_with_accel > time_without_accel

    def test_scanline_power_command(self):
        """Test timing estimation for ScanLinePowerCommand."""
        ops = Ops()
        ops.scan_to(x=100, y=0, z=0, power_values=bytearray([100] * 100))
        # ScanLinePowerCommand is a cutting command
        # Distance = 100mm
        # At 1000mm/min = 16.67mm/s, time = 100/16.67 = 6s + acceleration
        actual_time = ops.estimate_time()
        # Should be around 6.02s with acceleration
        assert 6.0 < actual_time < 6.05

    def test_mixed_commands(self):
        """Test timing estimation for mixed command types."""
        ops = Ops()
        ops.move_to(0, 0, 0)  # Initial position
        ops.line_to(10, 0, 0)  # 10mm cut
        ops.move_to(10, 10, 0)  # 10mm travel
        ops.line_to(0, 10, 0)  # 10mm cut
        ops.move_to(0, 0, 0)  # 14.14mm travel (diagonal)

        # Cut movements: 20mm total at 1000mm/min = 16.67mm/s = 1.2s
        # Travel movements: 24.14mm total at 3000mm/min = 50mm/s = 0.48s
        # Plus acceleration effects
        actual_time = ops.estimate_time()
        # Should be around 1.73s with acceleration
        assert 1.7 < actual_time < 1.8

    def test_ops_integration(self):
        """Test that Ops.estimate_time() produces consistent results."""
        ops = Ops()
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.line_to(10, 10)

        ops_time = ops.estimate_time()
        assert ops_time > 0

    def test_negligible_movement(self):
        """Test that very small movements are skipped."""
        ops = Ops()
        ops.line_to(0.000001, 0, 0)  # Very small movement
        actual_time = ops.estimate_time()
        # Very small movements should have minimal time
        assert actual_time < 0.001  # Should be very small

    def test_triangular_velocity_profile(self):
        """Test timing estimation when full speed cannot be reached."""
        ops = Ops()
        ops.line_to(1, 0, 0)  # Very short distance
        # With high acceleration, full speed won't be reached
        # Should use triangular velocity profile
        time_with_high_accel = ops.estimate_time(acceleration=10000.0)
        time_with_low_accel = ops.estimate_time(acceleration=100.0)
        # Higher acceleration should result in shorter time
        assert time_with_high_accel < time_with_low_accel

    def test_estimate_time_does_not_mutate_commands(self):
        """Test that estimate_time does not set .state on commands."""
        ops = Ops()
        ops.set_cut_speed(500)
        ops.line_to(10, 0, 0)

        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                assert ops.inspect(i).state is None

        ops.estimate_time()

        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                assert ops.inspect(i).state is None

    def test_estimate_time_caching(self):
        """Test that Ops.estimate_time() caches results."""
        ops = Ops()
        ops.move_to(0, 0)
        ops.line_to(100, 0)

        time1 = ops.estimate_time()
        time2 = ops.estimate_time()
        assert time1 == time2

    def test_cache_invalidated_on_add(self):
        """Test that adding a command invalidates
        the time cache."""
        ops = Ops()
        ops.line_to(10, 0, 0)
        time_before = ops.estimate_time()

        ops.line_to(20, 0, 0)
        time_after = ops.estimate_time()
        assert time_after != time_before

    def test_cache_invalidated_on_clear(self):
        """Test that clearing commands invalidates
        the time cache."""
        ops = Ops()
        ops.line_to(10, 0, 0)
        ops.estimate_time()

        ops.clear()
        assert ops.estimate_time() == 0.0

    def test_cache_invalidated_on_replace_all(self):
        """Test that replace_all invalidates
        the time cache."""
        ops = Ops()
        ops.line_to(10, 0, 0)
        time_before = ops.estimate_time()

        tmp = Ops()
        tmp.move_to(5, 5, 0)
        ops.replace_all(tmp)
        time_after = ops.estimate_time()
        assert time_after != time_before

    def test_cache_keyed_on_params(self):
        """Test that different machine parameters cause recomputation."""
        ops = Ops()
        ops.line_to(100, 0, 0)

        time_fast = ops.estimate_time(default_cut_speed=2000.0)
        time_slow = ops.estimate_time(default_cut_speed=500.0)
        assert time_fast < time_slow

    def test_cache_preserved_on_copy(self):
        """Test that copy() preserves the cache state."""
        ops = Ops()
        ops.line_to(100, 0, 0)
        ops.estimate_time()

        copied = ops.copy()
        assert copied.estimate_time() == ops.estimate_time()

    def test_cache_after_transform(self):
        """Test that transform() invalidates the cache."""
        ops = Ops()
        ops.line_to(100, 0, 0)
        time_before = ops.estimate_time()

        ops.translate(10, 10)
        time_after = ops.estimate_time()
        assert time_after != time_before

    def test_cache_after_extend(self):
        """Test that extend() invalidates the time cache."""
        ops1 = Ops()
        ops1.line_to(100, 0, 0)
        time_before = ops1.estimate_time()

        ops2 = Ops()
        ops2.move_to(50, 50)
        ops1.extend(ops2)
        time_after = ops1.estimate_time()
        assert time_after != time_before


class TestCommandTimes:
    """Test cases for per-command timing estimation."""

    def test_empty_commands(self):
        """Test that empty command list returns empty list."""
        ops = Ops()
        assert ops.estimate_command_times() == []

    def test_length_matches_commands(self):
        """Test that result length equals number of commands."""
        ops = Ops()
        ops.move_to(10, 10, 0)
        ops.set_cut_speed(500)
        ops.line_to(20, 0, 0)
        ops.set_power(0.5)
        ops.line_to(30, 0, 0)
        times = ops.estimate_command_times()
        assert len(times) == ops.len()

    def test_sum_equals_estimate_time(self):
        """Test that sum of command times equals estimate_time()."""
        ops = Ops()
        ops.move_to(10, 10, 0)
        ops.set_cut_speed(600)
        ops.line_to(50, 0, 0)
        ops.set_travel_speed(1200)
        ops.move_to(50, 50, 0)
        ops.line_to(0, 0, 0)
        total = ops.estimate_time()
        times = ops.estimate_command_times()
        assert abs(sum(times) - total) < 1e-10

    def test_state_commands_zero_time(self):
        """Test that state-setting commands have zero time."""
        ops = Ops()
        ops.set_cut_speed(500)
        ops.set_travel_speed(3000)
        ops.set_power(0.8)
        times = ops.estimate_command_times()
        assert all(t == 0.0 for t in times)

    def test_single_move_command(self):
        """Test per-command time for a single move command."""
        ops = Ops()
        ops.move_to(10, 10, 0)
        times = ops.estimate_command_times()
        assert len(times) == 1
        assert times[0] > 0

    def test_single_line_command_no_accel(self):
        """Test per-command time for a line without acceleration."""
        ops = Ops()
        ops.line_to(10, 0, 0)
        times = ops.estimate_command_times(acceleration=0)
        assert len(times) == 1
        expected = 0.6  # 10mm / (1000mm/min / 60) = 0.6s
        assert abs(times[0] - expected) < 0.01

    def test_speed_change_affects_subsequent(self):
        """Test that set_cut_speed only affects later commands."""
        ops = Ops()
        ops.line_to(60, 0, 0)
        ops.set_cut_speed(600)  # 10mm/s
        ops.line_to(120, 0, 0)
        times = ops.estimate_command_times(acceleration=0)
        assert len(times) == 3
        assert times[1] == 0.0  # set_cut_speed is zero
        # First 60mm at default 1000mm/min, second 60mm at 600mm/min
        assert times[0] < times[2]

    def test_move_vs_line_speeds(self):
        """Test that moves use travel speed, lines use cut speed."""
        ops = Ops()
        ops.move_to(100, 0, 0)
        ops.line_to(200, 0, 0)
        times = ops.estimate_command_times(acceleration=0)
        assert len(times) == 2
        # travel_speed=3000 is 3x faster than cut_speed=1000
        assert abs(times[0] - 2.0) < 0.01  # 100/(3000/60)=2s
        assert abs(times[1] - 6.0) < 0.01  # 100/(1000/60)=6s

    def test_custom_speeds(self):
        """Test per-command times with custom speed arguments."""
        ops = Ops()
        ops.line_to(60, 0, 0)
        times = ops.estimate_command_times(default_cut_speed=1200.0, acceleration=0)
        assert len(times) == 1
        expected = 3.0  # 60mm / (1200mm/min / 60) = 3s
        assert abs(times[0] - expected) < 0.01

    def test_negligible_movement_zero_time(self):
        """Test that negligible movements produce zero time."""
        ops = Ops()
        ops.line_to(0.000001, 0, 0)
        times = ops.estimate_command_times()
        assert len(times) == 1
        assert times[0] < 0.001

    def test_scanline_command(self):
        """Test per-command time for a scanline command."""
        ops = Ops()
        ops.scan_to(x=100, y=0, z=0, power_values=bytearray([100] * 100))
        times = ops.estimate_command_times(acceleration=0)
        assert len(times) == 1
        expected = 6.0  # 100mm / (1000mm/min / 60) = 6s
        assert abs(times[0] - expected) < 0.01

    def test_mixed_commands_sum(self):
        """Test sum of mixed command types equals estimate_time."""
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(10, 10, 0)
        ops.line_to(0, 10, 0)
        ops.move_to(0, 0, 0)
        total = ops.estimate_time()
        times = ops.estimate_command_times()
        assert abs(sum(times) - total) < 1e-10

    def test_acceleration_effect(self):
        """Test that acceleration increases move time."""
        ops = Ops()
        ops.line_to(10, 0, 0)
        times_no_accel = ops.estimate_command_times(acceleration=0)
        times_accel = ops.estimate_command_times(acceleration=1000.0)
        assert times_accel[0] > times_no_accel[0]

    def test_does_not_mutate_commands(self):
        """Test that estimate_command_times does not set .state."""
        ops = Ops()
        ops.set_cut_speed(500)
        ops.line_to(10, 0, 0)

        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                assert ops.inspect(i).state is None

        ops.estimate_command_times()

        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                assert ops.inspect(i).state is None

    def test_sum_equals_estimate_time_complex(self):
        """Test sum equality with speed changes, multiple moves."""
        ops = Ops()
        ops.move_to(0, 0)
        ops.set_cut_speed(500)
        ops.line_to(50, 0)
        ops.set_power(0.8)
        ops.line_to(50, 50)
        ops.set_travel_speed(2000)
        ops.move_to(0, 0)
        ops.set_cut_speed(1000)
        ops.line_to(100, 100)
        total = ops.estimate_time(
            default_cut_speed=1000.0,
            default_travel_speed=3000.0,
            acceleration=500.0,
        )
        times = ops.estimate_command_times(
            default_cut_speed=1000.0,
            default_travel_speed=3000.0,
            acceleration=500.0,
        )
        assert len(times) == ops.len()
        assert abs(sum(times) - total) < 1e-10

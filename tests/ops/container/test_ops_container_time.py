"""
Tests for timing estimation functionality.
"""

import math

import pytest

from raygeo.ops import Ops
from raygeo.ops.state import AirAssistMode
from raygeo.ops.types import CommandCategory

# --- estimate_time: basic / edge cases ---


def test_estimate_time_empty():
    ops = Ops()
    assert ops.estimate_time() == 0.0


def test_estimate_time_single_move():
    ops = Ops()
    ops.move_to(10, 10, 0)
    actual_time = ops.estimate_time()
    assert 0.3 < actual_time < 0.4


def test_estimate_time_single_line():
    ops = Ops()
    ops.line_to(10, 0, 0)
    actual_time = ops.estimate_time()
    assert 0.6 < actual_time < 0.65


def test_estimate_time_custom_speeds():
    ops = Ops()
    ops.line_to(60, 0, 0)
    actual_time = ops.estimate_time(default_feed_rate=1200.0)
    assert 3.0 < actual_time < 3.05


def test_estimate_time_speed_commands():
    ops = Ops()
    ops.set_feed_rate(600)  # 10mm/s
    ops.line_to(50, 0, 0)  # 5s at 10mm/s
    ops.set_rapid_rate(1200)  # 20mm/s
    ops.move_to(50, 50, 0)  # 2.5s at 20mm/s
    actual_time = ops.estimate_time()
    assert 7.5 < actual_time < 7.55


def test_estimate_time_acceleration_disabled():
    ops = Ops()
    ops.line_to(10, 0, 0)
    actual_time = ops.estimate_time(acceleration=0)
    expected_time = 0.6
    assert abs(actual_time - expected_time) < 0.01


def test_estimate_time_acceleration_enabled():
    ops = Ops()
    ops.line_to(10, 0, 0)
    time_with_accel = ops.estimate_time(acceleration=1000.0)
    time_without_accel = ops.estimate_time(acceleration=0.0)
    assert time_with_accel > time_without_accel


def test_estimate_time_scanline_power_command():
    ops = Ops()
    ops.scan_to(x=100, y=0, z=0, power_values=bytearray([100] * 100))
    actual_time = ops.estimate_time()
    assert 6.0 < actual_time < 6.05


def test_estimate_time_mixed_commands():
    ops = Ops()
    ops.move_to(0, 0, 0)  # Initial position
    ops.line_to(10, 0, 0)  # 10mm cut
    ops.move_to(10, 10, 0)  # 10mm travel
    ops.line_to(0, 10, 0)  # 10mm cut
    ops.move_to(0, 0, 0)  # 14.14mm travel (diagonal)
    actual_time = ops.estimate_time()
    assert 1.7 < actual_time < 1.8


def test_estimate_time_ops_integration():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    ops_time = ops.estimate_time()
    assert ops_time > 0


def test_estimate_time_negligible_movement():
    ops = Ops()
    ops.line_to(0.000001, 0, 0)
    actual_time = ops.estimate_time()
    assert actual_time < 0.001


def test_estimate_time_triangular_velocity_profile():
    ops = Ops()
    ops.line_to(1, 0, 0)
    time_with_high_accel = ops.estimate_time(acceleration=10000.0)
    time_with_low_accel = ops.estimate_time(acceleration=100.0)
    assert time_with_high_accel < time_with_low_accel


def test_estimate_time_does_not_mutate_commands():
    ops = Ops()
    ops.set_feed_rate(500)
    ops.line_to(10, 0, 0)

    for i in range(ops.len()):
        if ops.category(i) == CommandCategory.MOVING:
            assert ops.inspect(i).state is None

    ops.estimate_time()

    for i in range(ops.len()):
        if ops.category(i) == CommandCategory.MOVING:
            assert ops.inspect(i).state is None


def test_estimate_time_caching():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(100, 0)
    time1 = ops.estimate_time()
    time2 = ops.estimate_time()
    assert time1 == time2


def test_estimate_time_cache_invalidated_on_add():
    ops = Ops()
    ops.line_to(10, 0, 0)
    time_before = ops.estimate_time()
    ops.line_to(20, 0, 0)
    time_after = ops.estimate_time()
    assert time_after != time_before


def test_estimate_time_cache_invalidated_on_clear():
    ops = Ops()
    ops.line_to(10, 0, 0)
    ops.estimate_time()
    ops.clear()
    assert ops.estimate_time() == 0.0


def test_estimate_time_cache_invalidated_on_replace_all():
    ops = Ops()
    ops.line_to(10, 0, 0)
    time_before = ops.estimate_time()
    tmp = Ops()
    tmp.move_to(5, 5, 0)
    ops.replace_all(tmp)
    time_after = ops.estimate_time()
    assert time_after != time_before


def test_estimate_time_cache_keyed_on_params():
    ops = Ops()
    ops.line_to(100, 0, 0)
    time_fast = ops.estimate_time(default_feed_rate=2000.0)
    time_slow = ops.estimate_time(default_feed_rate=500.0)
    assert time_fast < time_slow


def test_estimate_time_cache_preserved_on_copy():
    ops = Ops()
    ops.line_to(100, 0, 0)
    ops.estimate_time()
    copied = ops.copy()
    assert copied.estimate_time() == ops.estimate_time()


def test_estimate_time_cache_after_transform():
    ops = Ops()
    ops.line_to(100, 0, 0)
    time_before = ops.estimate_time()
    ops.translate(10, 10)
    time_after = ops.estimate_time()
    assert time_after != time_before


def test_estimate_time_cache_after_extend():
    ops1 = Ops()
    ops1.line_to(100, 0, 0)
    time_before = ops1.estimate_time()
    ops2 = Ops()
    ops2.move_to(50, 50)
    ops1.extend(ops2)
    time_after = ops1.estimate_time()
    assert time_after != time_before


# --- estimate_command_times ---


def test_estimate_command_times_empty():
    ops = Ops()
    assert ops.estimate_command_times() == []


def test_estimate_command_times_length_matches_commands():
    ops = Ops()
    ops.move_to(10, 10, 0)
    ops.set_feed_rate(500)
    ops.line_to(20, 0, 0)
    ops.set_power(0.5)
    ops.line_to(30, 0, 0)
    times = ops.estimate_command_times()
    assert len(times) == ops.len()


def test_estimate_command_times_sum_equals_estimate_time():
    ops = Ops()
    ops.move_to(10, 10, 0)
    ops.set_feed_rate(600)
    ops.line_to(50, 0, 0)
    ops.set_rapid_rate(1200)
    ops.move_to(50, 50, 0)
    ops.line_to(0, 0, 0)
    total = ops.estimate_time()
    times = ops.estimate_command_times()
    assert abs(sum(times) - total) < 1e-10


def test_estimate_command_times_state_commands_zero():
    ops = Ops()
    ops.set_feed_rate(500)
    ops.set_rapid_rate(3000)
    ops.set_power(0.8)
    times = ops.estimate_command_times()
    assert all(t == 0.0 for t in times)


def test_estimate_command_times_single_move():
    ops = Ops()
    ops.move_to(10, 10, 0)
    times = ops.estimate_command_times()
    assert len(times) == 1
    assert times[0] > 0


def test_estimate_command_times_single_line_no_accel():
    ops = Ops()
    ops.line_to(10, 0, 0)
    times = ops.estimate_command_times(acceleration=0)
    assert len(times) == 1
    expected = 0.6
    assert abs(times[0] - expected) < 0.01


def test_estimate_command_times_speed_change_affects_subsequent():
    ops = Ops()
    ops.line_to(60, 0, 0)
    ops.set_feed_rate(600)
    ops.line_to(120, 0, 0)
    times = ops.estimate_command_times(acceleration=0)
    assert len(times) == 3
    assert times[1] == 0.0
    assert times[0] < times[2]


def test_estimate_command_times_move_vs_line():
    ops = Ops()
    ops.move_to(100, 0, 0)
    ops.line_to(200, 0, 0)
    times = ops.estimate_command_times(acceleration=0)
    assert len(times) == 2
    assert abs(times[0] - 2.0) < 0.01
    assert abs(times[1] - 6.0) < 0.01


def test_estimate_command_times_custom_speeds():
    ops = Ops()
    ops.line_to(60, 0, 0)
    times = ops.estimate_command_times(
        default_feed_rate=1200.0, acceleration=0
    )
    assert len(times) == 1
    expected = 3.0
    assert abs(times[0] - expected) < 0.01


def test_estimate_command_times_negligible_movement():
    ops = Ops()
    ops.line_to(0.000001, 0, 0)
    times = ops.estimate_command_times()
    assert len(times) == 1
    assert times[0] < 0.001


def test_estimate_command_times_scanline():
    ops = Ops()
    ops.scan_to(x=100, y=0, z=0, power_values=bytearray([100] * 100))
    times = ops.estimate_command_times(acceleration=0)
    assert len(times) == 1
    expected = 6.0
    assert abs(times[0] - expected) < 0.01


def test_estimate_command_times_mixed_sum():
    ops = Ops()
    ops.move_to(0, 0, 0)
    ops.line_to(10, 0, 0)
    ops.move_to(10, 10, 0)
    ops.line_to(0, 10, 0)
    ops.move_to(0, 0, 0)
    total = ops.estimate_time()
    times = ops.estimate_command_times()
    assert abs(sum(times) - total) < 1e-10


def test_estimate_command_times_acceleration_effect():
    ops = Ops()
    ops.line_to(10, 0, 0)
    times_no_accel = ops.estimate_command_times(acceleration=0)
    times_accel = ops.estimate_command_times(acceleration=1000.0)
    assert times_accel[0] > times_no_accel[0]


def test_estimate_command_times_does_not_mutate():
    ops = Ops()
    ops.set_feed_rate(500)
    ops.line_to(10, 0, 0)

    for i in range(ops.len()):
        if ops.category(i) == CommandCategory.MOVING:
            assert ops.inspect(i).state is None

    ops.estimate_command_times()

    for i in range(ops.len()):
        if ops.category(i) == CommandCategory.MOVING:
            assert ops.inspect(i).state is None


def test_estimate_command_times_sum_complex():
    ops = Ops()
    ops.move_to(0, 0)
    ops.set_feed_rate(500)
    ops.line_to(50, 0)
    ops.set_power(0.8)
    ops.line_to(50, 50)
    ops.set_rapid_rate(2000)
    ops.move_to(0, 0)
    ops.set_feed_rate(1000)
    ops.line_to(100, 100)
    total = ops.estimate_time(
        default_feed_rate=1000.0,
        default_rapid_rate=3000.0,
        acceleration=500.0,
    )
    times = ops.estimate_command_times(
        default_feed_rate=1000.0,
        default_rapid_rate=3000.0,
        acceleration=500.0,
    )
    assert len(times) == ops.len()
    assert abs(sum(times) - total) < 1e-10


# --- estimate_time: top-level scenario tests ---


def test_estimate_time_basic():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(100, 0)  # 100mm cut
    ops.move_to(0, 100)  # 141.42mm travel
    ops.line_to(100, 100)  # 100mm cut
    time_est = ops.estimate_time(acceleration=0)
    expected_time = 6.0 + 2.828 + 6.0
    assert time_est == pytest.approx(expected_time, rel=1e-3)


def test_estimate_time_with_custom_speeds():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(60, 0)  # 60mm cut
    ops.move_to(0, 80)  # 100mm travel
    time_est = ops.estimate_time(
        default_feed_rate=1200.0, default_rapid_rate=2400.0, acceleration=0
    )
    expected_time = 3.0 + 2.5
    assert time_est == pytest.approx(expected_time, rel=1e-3)


def test_estimate_time_with_state_commands():
    ops = Ops()
    ops.move_to(0, 0)
    ops.set_feed_rate(2000)  # Faster cutting speed
    ops.line_to(100, 0)  # 100mm cut at 2000mm/min
    ops.set_rapid_rate(6000)  # Faster travel speed
    ops.move_to(0, 100)  # 141.42mm travel at 6000mm/min
    time_est = ops.estimate_time(acceleration=0)
    expected_time = 3.0 + 1.414
    assert time_est == pytest.approx(expected_time, rel=1e-3)


def test_estimate_time_with_acceleration():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)  # Short movement
    time_est_no_accel = ops.estimate_time(acceleration=0)
    time_est_with_accel = ops.estimate_time(acceleration=1000)
    assert time_est_with_accel > time_est_no_accel


def test_estimate_time_with_arc():
    ops = Ops()
    ops.move_to(10, 0)
    ops.arc_to(0, 10, i=-10, j=0, clockwise=False)
    time_est = ops.estimate_time(acceleration=0)
    arc_len = math.pi / 2.0 * 10.0
    expected_time = arc_len / 1000 * 60 + 10.0 / 3000 * 60
    assert time_est == pytest.approx(expected_time, rel=1e-3)


def test_estimate_time_ignores_state_commands():
    ops = Ops()
    ops.move_to(0, 0)
    ops.set_power(0.5)
    ops.set_feed_rate(1000)
    ops.set_air_assist(AirAssistMode.ON)
    ops.line_to(60, 0)
    time_est = ops.estimate_time(acceleration=0)
    expected_time = 60.0 / 1000 * 60
    assert time_est == pytest.approx(expected_time, rel=1e-3)


def test_estimate_time_with_scanline():
    ops = Ops()
    ops.move_to(0, 50)
    ops.scan_to(100, 50, 0, bytearray([100] * 100))
    time_est = ops.estimate_time(acceleration=0)
    expected_time = 100.0 / 1000 * 60 + 50.0 / 3000 * 60
    assert time_est == pytest.approx(expected_time, rel=1e-3)


def test_estimate_time_complex_path():
    ops = Ops()
    ops.move_to(0, 0)
    ops.set_feed_rate(1500)
    ops.line_to(50, 0)
    ops.line_to(50, 50)
    ops.line_to(0, 50)
    ops.line_to(0, 0)
    ops.set_rapid_rate(3000)
    ops.move_to(100, 0)
    ops.set_feed_rate(2000)
    ops.line_to(150, 0)
    ops.line_to(150, 50)
    time_est = ops.estimate_time(acceleration=0)
    expected_time = 8.0 + 2.0 + 3.0
    assert time_est == pytest.approx(expected_time, rel=1e-3)

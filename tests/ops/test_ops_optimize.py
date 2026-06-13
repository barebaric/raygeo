import pytest

from raygeo.ops import Ops
from raygeo.ops.types import CommandCategory, CommandType


def _make_seg(start, end):
    ops = Ops()
    ops.move_to(*start)
    ops.line_to(*end)
    return ops


def _travel_distance(ops):
    ops.preload_state()
    return ops.distance() - ops.cut_distance()


def _count_cuts(ops):
    return sum(1 for i in range(ops.len()) if ops.is_cutting(i))


def _count_commands(ops, ct):
    return sum(1 for i in range(ops.len()) if ops.command_type(i) == ct)


def _endpoint_sequence(ops):
    return [ops.endpoint(i) for i in range(ops.len())]


class TestOptimizeEmpty:
    def test_empty_ops(self):
        ops = Ops()
        ops.optimize_travel()
        assert ops.is_empty()

    def test_empty_ops_with_flip(self):
        ops = Ops()
        ops.optimize_travel(allow_flip=True)
        assert ops.is_empty()

    def test_empty_ops_with_progress_cb(self):
        reported = []

        def cb(progress, message):
            reported.append((progress, message))

        ops = Ops()
        ops.optimize_travel(progress_cb=cb)
        assert ops.is_empty()


class TestOptimizeSingleSegment:
    def test_single_move_line(self):
        ops = Ops()
        ops.move_to(0, 0)
        ops.line_to(10, 10)
        original_len = ops.len()
        ops.optimize_travel()
        assert ops.len() == original_len

    def test_single_move_arc(self):
        ops = Ops()
        ops.move_to(0, 0)
        ops.arc_to(10, 0, 5, 0, False)
        original_len = ops.len()
        ops.optimize_travel()
        assert ops.len() == original_len

    def test_single_move_bezier(self):
        ops = Ops()
        ops.move_to(0, 0)
        ops.bezier_to((5, 5, 0), (10, 5, 0), (15, 0, 0))
        original_len = ops.len()
        ops.optimize_travel()
        assert ops.len() == original_len

    def test_single_scanline(self):
        ops = Ops()
        ops.move_to(0, 0)
        ops.scan_to(10, 0, 0, power_values=bytearray([100, 200, 100]))
        original_len = ops.len()
        ops.optimize_travel()
        assert ops.len() == original_len


class TestOptimizeTwoSegments:
    def test_reorder_two_segments(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        ops.optimize_travel()
        assert _count_cuts(ops) == 2

    def test_two_segments_flip(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(10, 10)
        ops.line_to(10, 0)
        ops.optimize_travel()
        assert _count_cuts(ops) == 2


class TestOptimizeTravelReduction:
    def test_travel_is_reduced(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        ops.move_to(110, 100)
        ops.line_to(110, 110)

        ops_copy = ops.copy()
        travel_before = _travel_distance(ops_copy)

        ops.optimize_travel()
        travel_after = _travel_distance(ops)

        assert travel_after < travel_before

    def test_cut_commands_preserved(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        cuts_before = _count_cuts(ops)
        ops.optimize_travel()
        cuts_after = _count_cuts(ops)
        assert cuts_after == cuts_before

    def test_already_optimal_path(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        travel_before = _travel_distance(ops.copy())
        ops.optimize_travel()
        travel_after = _travel_distance(ops)
        assert travel_after <= travel_before + 1e-6


class TestOptimizeAllowFlip:
    def test_flip_disabled_preserves_direction(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(10, 10)
        ops.line_to(10, 0)
        ops.optimize_travel(allow_flip=False)
        assert _count_cuts(ops) == 2

    def test_flip_enabled_by_default(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(20, 0)
        ops.scan_to(10, 0, 0, power_values=bytearray([10, 20, 30]))
        ops.optimize_travel(allow_flip=True)
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 1
        move_idx = scan_indices[0] - 1
        assert ops.command_type(move_idx) == CommandType.MOVE_TO
        assert ops.endpoint(move_idx) == pytest.approx((10.0, 0.0, 0.0))


class TestOptimizePreserveFirst:
    def test_preserve_first_keeps_first_workpiece(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.workpiece_start("wp-far")
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.workpiece_end("wp-far")
        ops.workpiece_start("wp-near")
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.workpiece_end("wp-near")
        ops.optimize_travel(preserve_first=True)
        wp_order = []
        for i in range(ops.len()):
            if ops.command_type(i) == CommandType.WORKPIECE_START:
                wp_order.append(ops.workpiece_uid(i))
        assert wp_order[0] == "wp-far"


class TestOptimizePreserveOrder:
    def test_preserve_order_keeps_specified_workpieces(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.workpiece_start("wp-a")
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.workpiece_end("wp-a")
        ops.workpiece_start("wp-b")
        ops.move_to(200, 200)
        ops.line_to(210, 200)
        ops.workpiece_end("wp-b")
        ops.workpiece_start("wp-c")
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        ops.workpiece_end("wp-c")
        ops.optimize_travel(preserve_order=["wp-b"])
        wp_order = []
        for i in range(ops.len()):
            if ops.command_type(i) == CommandType.WORKPIECE_START:
                wp_order.append(ops.workpiece_uid(i))
        assert "wp-b" in wp_order
        b_idx = wp_order.index("wp-b")
        assert b_idx == 1


class TestOptimizeWorkpieceLevel:
    def test_workpiece_reorder(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.workpiece_start("wp-a")
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.workpiece_end("wp-a")
        ops.workpiece_start("wp-c")
        ops.move_to(200, 200)
        ops.line_to(210, 200)
        ops.workpiece_end("wp-c")
        ops.workpiece_start("wp-b")
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        ops.workpiece_end("wp-b")
        ops.optimize_travel()
        wp_order = []
        for i in range(ops.len()):
            if ops.command_type(i) == CommandType.WORKPIECE_START:
                wp_order.append(ops.workpiece_uid(i))
        assert wp_order == ["wp-a", "wp-b", "wp-c"]

    def test_workpiece_markers_preserved(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.workpiece_start("wp-1")
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.workpiece_end("wp-1")
        ops.workpiece_start("wp-2")
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.workpiece_end("wp-2")
        ops.optimize_travel()
        start_count = _count_commands(ops, CommandType.WORKPIECE_START)
        end_count = _count_commands(ops, CommandType.WORKPIECE_END)
        assert start_count == 2
        assert end_count == 2

    def test_single_workpiece_no_reorder(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.workpiece_start("wp-only")
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.workpiece_end("wp-only")
        ops.optimize_travel()
        assert _count_commands(ops, CommandType.WORKPIECE_START) == 1

    def test_workpiece_flip(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.workpiece_start("wp-a")
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.workpiece_end("wp-a")
        ops.workpiece_start("wp-b")
        ops.move_to(10, 10)
        ops.line_to(10, 0)
        ops.workpiece_end("wp-b")
        ops.optimize_travel(allow_flip=True)
        assert _count_cuts(ops) == 2


class TestOptimizeStateBoundaries:
    def test_air_assist_boundary(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(0, 10)
        ops.line_to(10, 10)
        ops.enable_air_assist(True)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.move_to(100, 110)
        ops.line_to(110, 110)
        ops.optimize_travel()
        ops.preload_state()
        air_on_idx = -1
        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                if state.air_assist:
                    air_on_idx = i
                    break
        assert air_on_idx != -1
        for i in range(air_on_idx):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                assert not state.air_assist

    def test_power_change_boundary(self):
        ops = Ops()
        ops.set_power(0.4)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.set_power(0.9)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        ops.preload_state()
        powers = set()
        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                powers.add(round(state.power, 2))
        assert 0.4 in powers
        assert 0.9 in powers

    def test_cut_speed_boundary(self):
        ops = Ops()
        ops.set_cut_speed(500)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.set_cut_speed(2000)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        ops.preload_state()
        speeds = set()
        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                if state.cut_speed is not None:
                    speeds.add(state.cut_speed)
        assert 500 in speeds
        assert 2000 in speeds


class TestOptimizeMarkers:
    def test_job_start_marker_preserved(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.job_start()
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        marker_count = _count_commands(ops, CommandType.JOB_START)
        assert marker_count == 1

    def test_marker_acts_as_boundary(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.job_start()
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        ops.move_to(110, 100)
        ops.line_to(110, 110)
        ops.optimize_travel()
        marker_idx = -1
        for i in range(ops.len()):
            if ops.command_type(i) == CommandType.JOB_START:
                marker_idx = i
                break
        assert marker_idx != -1


class TestOptimizeScanline:
    def test_unsplit_scanline_flip(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(20, 0, 0)
        ops.scan_to(10, 0, 0, power_values=bytearray([10, 20, 30]))
        ops.optimize_travel()
        ops.preload_state()
        travel_after = _travel_distance(ops)
        assert travel_after == pytest.approx(0.0)

    def test_unsplit_scanline_geometry(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(20, 0, 0)
        ops.scan_to(10, 0, 0, power_values=bytearray([10, 20, 30]))
        ops.optimize_travel()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 1
        move_idx = scan_indices[0] - 1
        assert ops.endpoint(move_idx) == pytest.approx((10.0, 0.0, 0.0))
        assert ops.endpoint(scan_indices[0]) == pytest.approx((20.0, 0.0, 0.0))

    def test_unsplit_scanline_power_reversed(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(20, 0, 0)
        ops.scan_to(10, 0, 0, power_values=bytearray([10, 20, 30]))
        ops.optimize_travel()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 1
        assert bytearray(ops.scanline_data(scan_indices[0])) == bytearray(
            [30, 20, 10]
        )

    def test_split_scanline(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 5, 0)
        ops.line_to(108, 5, 0)
        ops.move_to(100, 5, 0)
        ops.scan_to(
            110, 5, 0, power_values=bytearray([50, 50, 0, 0, 0, 60, 60])
        )
        ops.optimize_travel()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 2

    def test_overscanned_scanline_not_split(self):
        ops = Ops()
        ops.set_power(1.0)
        start_pt = (0.0, 10.0, 0.0)
        end_pt = (20.0, 10.0, 0.0)
        power_values = bytearray([0, 0] + [50, 100, 150] + [0, 0])
        ops.move_to(*start_pt)
        ops.scan_to(*end_pt, power_values=power_values)
        ops.optimize_travel()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 1
        move_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.MOVE_TO
        ]
        assert len(move_indices) == 1
        assert ops.endpoint(move_indices[0]) == pytest.approx(start_pt)
        assert ops.endpoint(scan_indices[0]) == pytest.approx(end_pt)
        assert bytearray(ops.scanline_data(scan_indices[0])) == power_values

    def test_all_zero_scanline_still_present(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.scan_to(10, 0, 0, power_values=bytearray([0, 0, 0]))
        ops.move_to(20, 0, 0)
        ops.line_to(30, 0, 0)
        ops.optimize_travel()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 1

    def test_scanline_flip_preserves_state(self):
        ops = Ops()
        ops.set_power(0.85)
        ops.set_cut_speed(1234)
        ops.enable_air_assist(True)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(20, 0)
        ops.scan_to(10, 0, power_values=bytearray([10, 20, 30]))
        ops.optimize_travel()
        ops.preload_state()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 1
        scan_idx = scan_indices[0]
        move_idx = scan_idx - 1
        move_state = ops.preloaded_state(move_idx)
        assert move_state is not None
        assert move_state.power == pytest.approx(0.85)
        assert move_state.cut_speed == pytest.approx(1234)
        assert move_state.air_assist is True
        scan_state = ops.preloaded_state(scan_idx)
        assert scan_state is not None
        assert scan_state.power == pytest.approx(0.85)
        assert scan_state.cut_speed == pytest.approx(1234)
        assert scan_state.air_assist is True

    def test_scanline_split_preserves_state(self):
        ops = Ops()
        ops.set_power(0.77)
        ops.set_travel_speed(5678)
        ops.enable_air_assist(False)
        ops.move_to(0, 0)
        ops.scan_to(10, 0, power_values=bytearray([50, 50, 0, 0, 60, 60]))
        ops.move_to(100, 100)
        ops.line_to(101, 101)
        ops.optimize_travel()
        ops.preload_state()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 2
        for scan_idx in scan_indices:
            move_idx = scan_idx - 1
            move_state = ops.preloaded_state(move_idx)
            assert move_state is not None
            assert move_state.power == pytest.approx(0.77)
            assert move_state.travel_speed == pytest.approx(5678)
            assert move_state.air_assist is False
            scan_state = ops.preloaded_state(scan_idx)
            assert scan_state is not None
            assert scan_state.power == pytest.approx(0.77)
            assert scan_state.travel_speed == pytest.approx(5678)
            assert scan_state.air_assist is False

    def test_overscan_flip_preserves_state(self):
        ops = Ops()
        ops.set_power(0.66)
        ops.set_cut_speed(2000)
        ops.move_to(0, 0)
        ops.line_to(10, 10)
        start_pt = (35.0, 10.0, 0.0)
        end_pt = (15.0, 10.0, 0.0)
        power_values = bytearray([0, 0] + [100, 120, 140] + [0, 0])
        ops.move_to(*start_pt)
        ops.scan_to(*end_pt, power_values=power_values)
        ops.optimize_travel()
        ops.preload_state()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 1
        flipped_scan_idx = scan_indices[0]
        move_idx = flipped_scan_idx - 1
        move_state = ops.preloaded_state(move_idx)
        assert move_state is not None
        assert move_state.power == pytest.approx(0.66)
        assert move_state.cut_speed == pytest.approx(2000)
        scan_state = ops.preloaded_state(flipped_scan_idx)
        assert scan_state is not None
        assert scan_state.power == pytest.approx(0.66)
        assert scan_state.cut_speed == pytest.approx(2000)
        assert ops.endpoint(move_idx) == pytest.approx(end_pt)
        assert ops.endpoint(flipped_scan_idx) == pytest.approx(start_pt)
        assert (
            bytearray(ops.scanline_data(flipped_scan_idx))
            == power_values[::-1]
        )


class TestOptimizeBezier:
    def test_bezier_passes_through(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 0)
        ops.bezier_to((110, 10, 0), (120, 10, 0), (130, 0, 0))
        ops.optimize_travel()
        ops.preload_state()
        bezier_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.BEZIER_TO
        ]
        assert len(bezier_indices) == 1
        c1, c2 = ops.bezier_params(bezier_indices[0])
        assert c1 == (110, 10, 0)
        assert c2 == (120, 10, 0)
        assert ops.endpoint(bezier_indices[0]) == (130, 0, 0)

    def test_bezier_flip(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(30, 0)
        ops.bezier_to((25, 5, 0), (15, 5, 0), (10, 0, 0))
        ops.optimize_travel()
        ops.preload_state()
        bezier_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.BEZIER_TO
        ]
        assert len(bezier_indices) == 1
        idx = bezier_indices[0]
        assert ops.endpoint(idx) == pytest.approx((30, 0, 0))
        c1, c2 = ops.bezier_params(idx)
        assert c1 == pytest.approx((15, 5, 0))
        assert c2 == pytest.approx((25, 5, 0))

    def test_mixed_lines_and_bezier(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 100)
        ops.bezier_to((105, 105, 0), (115, 105, 0), (120, 100, 0))
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        ops.optimize_travel()
        ops.preload_state()
        travel_after = _travel_distance(ops)
        ops_unopt = Ops()
        ops_unopt.set_power(1.0)
        ops_unopt.move_to(0, 0)
        ops_unopt.line_to(10, 0)
        ops_unopt.move_to(100, 100)
        ops_unopt.bezier_to((105, 105, 0), (115, 105, 0), (120, 100, 0))
        ops_unopt.move_to(10, 0)
        ops_unopt.line_to(10, 10)
        travel_before = _travel_distance(ops_unopt)
        assert travel_after < travel_before
        bezier_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.BEZIER_TO
        ]
        assert len(bezier_indices) == 1
        line_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.LINE_TO
        ]
        assert len(line_indices) == 2


class TestOptimizeArc:
    def test_arc_passes_through(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.arc_to(10, 0, 5, 0, False)
        ops.optimize_travel()
        arc_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.ARC_TO
        ]
        assert len(arc_indices) == 1

    def test_arc_in_multi_segment(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(50, 50)
        ops.arc_to(60, 50, 5, 0, False)
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        ops.optimize_travel()
        arc_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.ARC_TO
        ]
        assert len(arc_indices) == 1


class TestOptimizeProgressCallback:
    def test_progress_callback_called(self):
        reported = []

        def cb(progress, message):
            reported.append((progress, message))

        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel(progress_cb=cb)
        assert len(reported) > 0

    def test_progress_values_monotonic(self):
        progresses = []

        def cb(progress, message):
            progresses.append(progress)

        ops = Ops()
        ops.set_power(1.0)
        for i in range(5):
            ops.move_to(i * 50, 0)
            ops.line_to(i * 50 + 10, 0)
        ops.optimize_travel(progress_cb=cb)
        for i in range(1, len(progresses)):
            assert progresses[i] >= progresses[i - 1] - 1e-9

    def test_progress_reaches_one(self):
        progresses = []

        def cb(progress, message):
            progresses.append(progress)

        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel(progress_cb=cb)
        assert any(abs(p - 1.0) < 1e-9 for p in progresses)


class TestOptimizeMultipleSegments:
    def test_many_segments_travel_reduced(self):
        ops = Ops()
        ops.set_power(1.0)
        segments = [
            ((0, 0), (10, 0)),
            ((200, 200), (210, 200)),
            ((10, 0), (10, 10)),
            ((210, 200), (210, 210)),
        ]
        for start, end in segments:
            ops.move_to(*start)
            ops.line_to(*end)
        travel_before = _travel_distance(ops.copy())
        ops.optimize_travel()
        travel_after = _travel_distance(ops)
        assert travel_after < travel_before

    def test_segments_count_preserved(self):
        ops = Ops()
        ops.set_power(1.0)
        for i in range(10):
            ops.move_to(i * 20, 0)
            ops.line_to(i * 20 + 10, 0)
        cuts_before = _count_cuts(ops)
        ops.optimize_travel()
        cuts_after = _count_cuts(ops)
        assert cuts_after == cuts_before

    def test_collinear_segments(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(30, 0)
        ops.line_to(40, 0)
        ops.move_to(10, 0)
        ops.line_to(20, 0)
        ops.optimize_travel()
        assert _count_cuts(ops) == 3


class TestOptimizeStateSynchronization:
    def test_power_sync_after_reorder(self):
        ops = Ops()
        ops.set_power(0.5)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.set_power(1.0)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        ops.preload_state()
        has_05 = False
        has_10 = False
        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                if abs(state.power - 0.5) < 0.01:
                    has_05 = True
                if abs(state.power - 1.0) < 0.01:
                    has_10 = True
        assert has_05
        assert has_10

    def test_air_assist_sync_after_reorder(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.enable_air_assist(True)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        ops.preload_state()
        air_states = set()
        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                air_states.add(state.air_assist)
        assert True in air_states
        assert False in air_states

    def test_travel_speed_sync_after_reorder(self):
        ops = Ops()
        ops.set_travel_speed(1000)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.set_travel_speed(5000)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        ops.preload_state()
        speeds = set()
        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                if state.travel_speed is not None:
                    speeds.add(state.travel_speed)
        assert 1000 in speeds
        assert 5000 in speeds

    def test_laser_uid_sync_after_reorder(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.set_laser("laser-a")
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.set_laser("laser-b")
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        ops.preload_state()
        lasers = set()
        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                if state.active_laser_uid is not None:
                    lasers.add(state.active_laser_uid)
        assert "laser-a" in lasers
        assert "laser-b" in lasers


class TestOptimizeTwoOptRefinement:
    def test_crossed_paths_uncrossed(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(1, 0)
        ops.move_to(10, 10)
        ops.line_to(11, 10)
        ops.move_to(2, 0)
        ops.line_to(1, 0)
        ops.move_to(11, 10)
        ops.line_to(12, 10)
        ops.optimize_travel()
        assert _count_cuts(ops) == 4

    def test_already_optimal_no_change(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(10, 0)
        ops.line_to(10, 10)
        ops.move_to(10, 10)
        ops.line_to(20, 10)
        travel_before = _travel_distance(ops.copy())
        ops.optimize_travel()
        travel_after = _travel_distance(ops)
        assert travel_after <= travel_before + 1e-6


class TestOptimizeEdgeCases:
    def test_only_state_commands(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.set_cut_speed(500)
        ops.optimize_travel()
        move_count = sum(
            1
            for i in range(ops.len())
            if ops.category(i) == CommandCategory.MOVING
        )
        assert move_count == 0

    def test_single_point_move(self):
        ops = Ops()
        ops.move_to(5, 5)
        ops.optimize_travel()
        assert ops.len() >= 1

    def test_overlapping_segments(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.optimize_travel()
        assert _count_cuts(ops) == 2

    def test_zero_length_segment(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(5, 5)
        ops.line_to(5, 5)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.optimize_travel()
        assert _count_cuts(ops) >= 1

    def test_large_coordinate_range(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(1, 0)
        ops.move_to(100000, 100000)
        ops.line_to(100001, 100000)
        ops.optimize_travel()
        assert _count_cuts(ops) == 2

    def test_negative_coordinates(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(-10, -10)
        ops.line_to(-5, -5)
        ops.move_to(-10, 10)
        ops.line_to(-5, 10)
        ops.optimize_travel()
        assert _count_cuts(ops) == 2


class TestOptimizeBothAPIs:
    def test_ops_method_api(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        assert _count_cuts(ops) == 2

    def test_module_function_api(self):
        import importlib

        mod = importlib.import_module("raygeo.ops.algo.optimize")
        optimize_travel = mod.optimize_travel

        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        optimize_travel(ops)
        assert _count_cuts(ops) == 2

    def test_both_apis_produce_same_result(self):
        import importlib

        mod = importlib.import_module("raygeo.ops.algo.optimize")
        optimize_fn = mod.optimize_travel

        ops1 = Ops()
        ops1.set_power(1.0)
        ops1.move_to(0, 0)
        ops1.line_to(10, 0)
        ops1.move_to(100, 100)
        ops1.line_to(110, 100)
        ops1.move_to(10, 0)
        ops1.line_to(10, 10)
        ops1.optimize_travel()

        ops2 = Ops()
        ops2.set_power(1.0)
        ops2.move_to(0, 0)
        ops2.line_to(10, 0)
        ops2.move_to(100, 100)
        ops2.line_to(110, 100)
        ops2.move_to(10, 0)
        ops2.line_to(10, 10)
        optimize_fn(ops2)

        assert ops1.len() == ops2.len()
        for i in range(ops1.len()):
            assert ops1.command_type(i) == ops2.command_type(i)
            assert ops1.endpoint(i) == pytest.approx(ops2.endpoint(i))


class TestOptimizeComplexScenarios:
    def test_state_change_and_scanlines(self):
        ops = Ops()
        ops.set_power(0.4)
        ops.move_to(0, 0)
        ops.scan_to(10, 0, power_values=bytearray([10]))
        ops.move_to(0, 10)
        ops.scan_to(10, 10, power_values=bytearray([20]))
        ops.set_power(0.9)
        ops.move_to(100, 100)
        ops.scan_to(110, 100, power_values=bytearray([30]))
        ops.move_to(100, 110)
        ops.scan_to(110, 110, power_values=bytearray([40]))
        ops.optimize_travel()
        ops.preload_state()
        power_change_idx = -1
        for i in range(ops.len()):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                if state.power == pytest.approx(0.9):
                    power_change_idx = i
                    break
        assert power_change_idx != -1
        for i in range(power_change_idx):
            if ops.category(i) == CommandCategory.MOVING:
                state = ops.preloaded_state(i)
                assert state is not None
                assert state.power == pytest.approx(0.4)

    def test_multiple_workpieces_with_states(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.workpiece_start("wp-a")
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.workpiece_end("wp-a")
        ops.enable_air_assist(True)
        ops.workpiece_start("wp-b")
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.workpiece_end("wp-b")
        ops.optimize_travel()
        ops.preload_state()
        wp_order = []
        for i in range(ops.len()):
            if ops.command_type(i) == CommandType.WORKPIECE_START:
                wp_order.append(ops.workpiece_uid(i))
        assert len(wp_order) == 2

    def test_many_small_segments(self):
        ops = Ops()
        ops.set_power(1.0)
        for i in range(20):
            x = i * 5
            ops.move_to(x, 0)
            ops.line_to(x + 3, 0)
        cuts_before = _count_cuts(ops)
        ops.optimize_travel()
        cuts_after = _count_cuts(ops)
        assert cuts_after == cuts_before

    def test_scanline_with_single_power_value(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(20, 0, 0)
        ops.scan_to(10, 0, 0, power_values=bytearray([100]))
        ops.optimize_travel()
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) == 1

    def test_adjacent_segments_no_travel(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(10, 0)
        ops.line_to(20, 0)
        ops.move_to(20, 0)
        ops.line_to(30, 0)
        travel_before = _travel_distance(ops.copy())
        ops.optimize_travel()
        travel_after = _travel_distance(ops)
        assert travel_after <= travel_before + 1e-6


class TestOptimizeMultiCommand:
    def test_segment_with_multiple_cuts(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.line_to(10, 10)
        ops.line_to(0, 10)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.line_to(110, 110)
        ops.optimize_travel()
        assert _count_cuts(ops) == 5

    def test_segment_with_arc_and_line(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.arc_to(10, 0, 5, 0, False)
        ops.line_to(10, 10)
        ops.move_to(100, 100)
        ops.line_to(110, 100)
        ops.optimize_travel()
        assert _count_cuts(ops) == 3

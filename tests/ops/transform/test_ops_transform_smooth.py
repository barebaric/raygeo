import math

from raygeo.ops import Ops
from raygeo.ops.types import CommandType


def assert_points_almost_equal(p1, p2, places=5, msg=None):
    assert abs(p1[0] - p2[0]) < 10 ** (-places), (
        f"{msg} (x): {p1[0]} != {p2[0]}"
    )
    assert abs(p1[1] - p2[1]) < 10 ** (-places), (
        f"{msg} (y): {p1[1]} != {p2[1]}"
    )
    assert abs(p1[2] - p2[2]) < 10 ** (-places), (
        f"{msg} (z): {p1[2]} != {p2[2]}"
    )


def distance_2d(p1, p2):
    return math.hypot(p1[0] - p2[0], p1[1] - p2[1])


class TestSmooth:
    def test_zero_amount_noop(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.smooth(0, 45)
        assert ops.len() == 2

    def test_empty_ops(self):
        ops = Ops()
        ops.smooth(50, 45)
        assert ops.is_empty()

    def test_single_segment_noop_too_short(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.smooth(50, 45)
        assert ops.len() == 1

    def test_open_path_subdivided(self):
        ops = Ops()
        ops.move_to(0, 0, 5)
        ops.line_to(50, 0, 5)
        ops.line_to(100, 50, 5)

        ops.smooth(50, 45)

        assert ops.len() > 3
        pts = [ops.endpoint(i) for i in range(ops.len())]
        assert_points_almost_equal(pts[0], (0, 0, 5))
        assert_points_almost_equal(pts[-1], (100, 50, 5))

    def test_corner_preserved(self):
        ops = Ops()
        ops.move_to(0, 50, 0)
        ops.line_to(50, 0, 0)
        ops.line_to(100, 50, 0)
        ops.line_to(150, 50, 0)

        ops.smooth(40, 95)

        pts = [ops.endpoint(i) for i in range(ops.len())]
        found_sharp = any(distance_2d(p, (50, 0, 0)) < 1e-5 for p in pts)
        assert found_sharp, "Sharp corner not preserved"

        found_dull = any(distance_2d(p, (100, 50, 0)) < 1e-5 for p in pts)
        assert not found_dull, "Dull corner not smoothed"

    def test_arcs_linearized_and_smoothed(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.arc_to(10, 10, 5, 0, True)

        ops.smooth(50, 45)

        assert ops.len() > 2
        assert ops.command_type(0) == CommandType.MOVE_TO
        assert all(
            ops.command_type(i) == CommandType.LINE_TO
            for i in range(1, ops.len())
        )

    def test_bezier_transferred_unchanged(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.bezier_to((10, 20, 0), (30, 20, 0), (40, 0, 0))

        ops.smooth(50, 45)

        bezier_indices = ops.indices_of(CommandType.BEZIER_TO)
        assert len(bezier_indices) == 1
        c1, c2 = ops.bezier_params(bezier_indices[0])
        assert c1 == (10, 20, 0)
        assert c2 == (30, 20, 0)
        assert ops.endpoint(bezier_indices[0]) == (40, 0, 0)

    def test_mixed_lines_and_bezier(self):
        ops = Ops()
        ops.set_power(1.0)

        ops.move_to(0, 0, 0)
        ops.line_to(50, 0, 0)
        ops.line_to(100, 50, 0)

        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.bezier_to((20, 10, 0), (30, 10, 0), (40, 0, 0))

        ops.smooth(50, 45)

        bezier_indices = ops.indices_of(CommandType.BEZIER_TO)
        assert len(bezier_indices) == 1
        line_count = len(ops.indices_of(CommandType.LINE_TO))
        assert line_count > 2

    def test_state_commands_preserved(self):
        ops = Ops()
        ops.set_power(0.5)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)

        ops.smooth(50, 45)

        power_indices = ops.indices_of(CommandType.SET_POWER)
        assert len(power_indices) == 1

    def test_multiple_segments(self):
        ops = Ops()
        for i in range(5):
            ops.move_to(i * 10, 0, 0)
            ops.line_to(i * 10 + 5, 5, 0)

        ops.smooth(50, 45)

        segments = list(ops.segment_indices())
        assert len(segments) == 5

    def test_z_coordinate_preserved(self):
        ops = Ops()
        ops.move_to(0, 0, 3)
        ops.line_to(50, 0, 3)
        ops.line_to(100, 50, 3)

        ops.smooth(50, 45)

        pts = [ops.endpoint(i) for i in range(ops.len())]
        assert_points_almost_equal(pts[0], (0, 0, 3))
        assert_points_almost_equal(pts[-1], (100, 50, 3))

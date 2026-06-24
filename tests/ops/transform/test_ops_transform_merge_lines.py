import math

from raygeo.ops import Ops
from raygeo.ops.types import CommandType


def _count(ops, ct):
    return len(ops.indices_of(ct))


class TestBasic:
    def test_empty_ops(self):
        ops = Ops()
        ops.merge_overlapping_lines(0.1)
        assert ops.is_empty()

    def test_no_duplicate_lines(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(20, 0)
        ops.line_to(30, 0)

        orig_moves = _count(ops, CommandType.MOVE_TO)
        orig_lines = _count(ops, CommandType.LINE_TO)

        ops.merge_overlapping_lines(0.1)

        assert _count(ops, CommandType.MOVE_TO) == orig_moves
        assert _count(ops, CommandType.LINE_TO) == orig_lines

    def test_identical_duplicate_lines_removed(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)

        ops.merge_overlapping_lines(0.1)

        assert _count(ops, CommandType.LINE_TO) == 1

    def test_opposite_direction_duplicate_lines_removed(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(10, 0)
        ops.line_to(0, 0)

        ops.merge_overlapping_lines(0.1)

        assert _count(ops, CommandType.LINE_TO) == 1


class TestTolerance:
    def test_tolerance_affects_merging(self):
        ops_tight = Ops()
        ops_tight.set_power(1.0)
        ops_tight.move_to(0, 0)
        ops_tight.line_to(10, 0)
        ops_tight.move_to(0, 0.05)
        ops_tight.line_to(10, 0.05)

        ops_tight.merge_overlapping_lines(0.01)
        assert _count(ops_tight, CommandType.LINE_TO) == 2

        ops_loose = Ops()
        ops_loose.set_power(1.0)
        ops_loose.move_to(0, 0)
        ops_loose.line_to(10, 0)
        ops_loose.move_to(0, 0.05)
        ops_loose.line_to(10, 0.05)

        ops_loose.merge_overlapping_lines(0.2)
        assert _count(ops_loose, CommandType.LINE_TO) == 1


class TestOverlapping:
    def test_overlapping_collinear_segments(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(5, 0)
        ops.line_to(15, 0)

        tol = 0.1
        ops.merge_overlapping_lines(tol)

        assert _count(ops, CommandType.LINE_TO) == 2
        assert math.isclose(ops.cut_distance(), 15.0 - tol)

    def test_perpendicular_lines_not_merged(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.move_to(5, -5)
        ops.line_to(5, 5)

        orig_lines = _count(ops, CommandType.LINE_TO)
        ops.merge_overlapping_lines(0.1)

        assert _count(ops, CommandType.LINE_TO) == orig_lines

    def test_adjacent_rectangles_shared_edge(self):
        ops = Ops()
        ops.set_power(1.0)

        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.line_to(10, 10)
        ops.line_to(0, 10)
        ops.line_to(0, 0)

        ops.move_to(10, 0)
        ops.line_to(20, 0)
        ops.line_to(20, 10)
        ops.line_to(10, 10)
        ops.line_to(10, 0)

        orig_lines = _count(ops, CommandType.LINE_TO)
        ops.merge_overlapping_lines(0.1)

        assert _count(ops, CommandType.LINE_TO) < orig_lines

    def test_triangle_shared_edge(self):
        ops = Ops()
        ops.set_power(1.0)

        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.line_to(5, 10)
        ops.line_to(0, 0)

        ops.move_to(10, 0)
        ops.line_to(0, 0)
        ops.line_to(5, -10)
        ops.line_to(10, 0)

        orig_lines = _count(ops, CommandType.LINE_TO)
        ops.merge_overlapping_lines(0.1)

        assert _count(ops, CommandType.LINE_TO) < orig_lines


class TestCurves:
    def test_bezier_passes_through_unchanged(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.bezier_to((3.0, 5.0, 0.0), (7.0, 5.0, 0.0), (10.0, 0.0, 0.0))

        ops.merge_overlapping_lines(0.1)

        bez = ops.indices_of(CommandType.BEZIER_TO)
        assert len(bez) == 1
        idx = bez[0]
        assert ops.endpoint(idx) == (10.0, 0.0, 0.0)
        c1, c2 = ops.bezier_params(idx)
        assert c1 == (3.0, 5.0, 0.0)
        assert c2 == (7.0, 5.0, 0.0)

    def test_mixed_lines_and_bezier(self):
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)
        ops.bezier_to((12.0, 5.0, 0.0), (18.0, 5.0, 0.0), (20.0, 0.0, 0.0))
        ops.line_to(30, 0)
        ops.move_to(0, 0)
        ops.line_to(10, 0)

        ops.merge_overlapping_lines(0.1)

        bez = ops.indices_of(CommandType.BEZIER_TO)
        assert len(bez) == 1
        idx = bez[0]
        c1, _ = ops.bezier_params(idx)
        assert c1 == (12.0, 5.0, 0.0)

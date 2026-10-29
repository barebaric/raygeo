"""Node editing: move, delete and insert nodes, line/curve, handles."""

import pytest

from raygeo.geo import Geometry

SQUARE = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]


def _polyline(points, close=False) -> Geometry:
    geo = Geometry()
    geo.move_to(*points[0])
    for p in points[1:]:
        geo.line_to(*p)
    if close:
        geo.line_to(*points[0])
    return geo


def _ends(geo: Geometry) -> list[tuple[float, float]]:
    return [(c.end[0], c.end[1]) for c in geo.data]


def _kinds(geo: Geometry) -> list[str]:
    return [type(c).__name__ for c in geo.data]


class TestMoveNode:
    def test_moves_an_interior_node(self):
        geo, index = _polyline(SQUARE).move_node(2, 12.0, 11.0)
        assert index == 2
        assert _ends(geo)[2] == (12.0, 11.0)
        assert _ends(geo)[1] == (10.0, 0.0)

    def test_closed_contour_moves_both_ends(self):
        geo, _ = _polyline(SQUARE, close=True).move_node(0, -1.0, -2.0)
        assert _ends(geo)[0] == (-1.0, -2.0)
        assert _ends(geo)[-1] == (-1.0, -2.0)
        geo, _ = _polyline(SQUARE, close=True).move_node(4, 1.0, 1.0)
        assert _ends(geo)[0] == (1.0, 1.0)

    def test_handles_follow_the_node(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 0, 3, 5, 7, 5)
        geo.bezier_to(20, 0, 13, -5, 17, -5)
        moved, _ = geo.move_node(1, 10.0, 2.0)
        first, second = moved.data[1], moved.data[2]
        assert first.control2[:2] == pytest.approx((7.0, 7.0))
        assert second.control1[:2] == pytest.approx((13.0, -3.0))
        assert first.control1[:2] == pytest.approx((3.0, 5.0))

    def test_adjacent_arc_becomes_bezier(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.arc_to(10, 0, 5, 0, clockwise=True)
        geo.line_to(10, -5)
        moved, index = geo.move_node(1, 11.0, 0.0)
        assert "Arc" not in _kinds(moved)
        assert _ends(moved)[index] == (11.0, 0.0)
        assert _ends(moved)[-1] == (10.0, -5.0)

    def test_out_of_range_is_a_no_op(self):
        geo = _polyline(SQUARE)
        moved, _ = geo.move_node(99, 1.0, 1.0)
        assert _ends(moved) == _ends(geo)


class TestDeleteNode:
    def test_interior_node_joins_lines(self):
        geo, index = _polyline(SQUARE).delete_node(1)
        assert _ends(geo) == [(0.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        assert _kinds(geo) == ["Move", "Line", "Line"]
        assert index == 1

    def test_join_keeps_outer_handles(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 0, 2, 4, 8, 4)
        geo.line_to(20, 0)
        joined, index = geo.delete_node(1)
        assert _kinds(joined) == ["Move", "Bezier"]
        assert joined.data[1].control1[:2] == pytest.approx((2.0, 4.0))
        assert joined.data[1].control2[:2] == pytest.approx((20.0, 0.0))
        assert index == 1

    def test_open_ends_are_dropped(self):
        geo, _ = _polyline(SQUARE).delete_node(3)
        assert _ends(geo) == SQUARE[:3]
        geo, _ = _polyline(SQUARE).delete_node(0)
        assert _ends(geo) == SQUARE[1:]
        assert _kinds(geo)[0] == "Move"

    def test_closed_start_node_rejoins_the_seam(self):
        geo, _ = _polyline(SQUARE, close=True).delete_node(0)
        assert _ends(geo)[0] == (10.0, 0.0)
        assert _ends(geo)[-1] == (10.0, 0.0)
        assert len(geo.data) == 4

    def test_closed_end_node_is_the_same_node(self):
        a, _ = _polyline(SQUARE, close=True).delete_node(0)
        b, _ = _polyline(SQUARE, close=True).delete_node(4)
        assert _ends(a) == _ends(b)

    def test_degenerate_contour_is_removed(self):
        geo = _polyline([(0, 0), (5, 0)])
        geo.extend(_polyline([(20, 20), (30, 20), (30, 30)]))
        result, _ = geo.delete_node(1)
        assert _ends(result) == [(20.0, 20.0), (30.0, 20.0), (30.0, 30.0)]

    def test_other_contours_are_untouched(self):
        geo = _polyline(SQUARE)
        geo.extend(_polyline([(20, 20), (30, 20)]))
        result, _ = geo.delete_node(2)
        assert _ends(result)[-2:] == [(20.0, 20.0), (30.0, 20.0)]


class TestInsertNode:
    def test_splits_a_line(self):
        geo, index = _polyline(SQUARE).insert_node(1, 0.25)
        assert index == 1
        assert _ends(geo)[1] == (2.5, 0.0)
        assert _ends(geo)[2] == (10.0, 0.0)
        assert len(geo.data) == 5

    def test_splits_a_bezier_on_the_curve(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 0, 0, 10, 10, 10)
        point = geo.get_point_at(1, 0.5)
        assert point is not None
        split, index = geo.insert_node(1, 0.5)
        assert _kinds(split) == ["Move", "Bezier", "Bezier"]
        assert _ends(split)[index] == pytest.approx(point[:2])
        assert split.get_point_at(2, 0.5) == pytest.approx(
            geo.get_point_at(1, 0.75)
        )

    def test_move_command_is_not_split(self):
        geo, _ = _polyline(SQUARE).insert_node(0, 0.5)
        assert len(geo.data) == 4


class TestSegmentCurve:
    def test_line_to_curve_and_back(self):
        curved, index = _polyline(SQUARE).set_segment_curve(1, True)
        assert _kinds(curved)[index] == "Bezier"
        bezier = curved.data[index]
        assert bezier.control1[:2] == pytest.approx((10 / 3, 0.0))
        assert bezier.control2[:2] == pytest.approx((20 / 3, 0.0))
        straight, _ = curved.set_segment_curve(index, False)
        assert _kinds(straight) == _kinds(_polyline(SQUARE))
        assert _ends(straight) == SQUARE

    def test_arc_to_curve(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.arc_to(10, 0, 5, 0, clockwise=True)
        curved, index = geo.set_segment_curve(1, True)
        assert "Arc" not in _kinds(curved)
        assert _ends(curved)[index] == (10.0, 0.0)


class TestMoveHandle:
    def test_moves_one_handle(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 0, 3, 5, 7, 5)
        moved = geo.move_handle(1, 2, 8.0, 9.0)
        assert moved.data[1].control2[:2] == pytest.approx((8.0, 9.0))
        assert moved.data[1].control1[:2] == pytest.approx((3.0, 5.0))

    def test_ignores_lines(self):
        geo = _polyline(SQUARE)
        assert _ends(geo.move_handle(1, 1, 5.0, 5.0)) == SQUARE


class TestContourInfo:
    def test_open_contour(self):
        assert _polyline(SQUARE).contour_info(2) == (0, 3, False)

    def test_closed_contour(self):
        geo = _polyline(SQUARE, close=True)
        assert geo.contour_info(4) == (0, 4, True)
        assert geo.contour_info(0) == (0, 4, True)

    def test_reports_the_contour_of_the_index(self):
        geo = _polyline(SQUARE)
        geo.extend(_polyline([(20, 20), (30, 20), (30, 30)]))
        assert geo.contour_info(0) == (0, 3, False)
        assert geo.contour_info(4) == (4, 6, False)

    def test_closure_tolerance_matches_the_node_operations(self):
        closed = Geometry()
        closed.move_to(0, 0)
        closed.line_to(10, 0)
        closed.line_to(5e-10, 0)
        assert closed.contour_info(2) == (0, 2, True)
        open_geo = Geometry()
        open_geo.move_to(0, 0)
        open_geo.line_to(10, 0)
        open_geo.line_to(1e-6, 0)
        assert open_geo.contour_info(2) == (0, 2, False)

    def test_leading_line_without_a_move(self):
        geo = Geometry()
        geo.line_to(5, 5)
        assert geo.contour_info(0) == (0, 0, False)

    def test_a_lone_move(self):
        geo = Geometry()
        geo.move_to(1, 1)
        assert geo.contour_info(0) == (0, 0, False)

    def test_out_of_range_is_degenerate(self):
        assert _polyline(SQUARE).contour_info(99) == (99, 99, False)

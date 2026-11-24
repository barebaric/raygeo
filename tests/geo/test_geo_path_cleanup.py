"""Contour-level path clean-up: close, join, de-duplicate and split."""

import math

import pytest

from raygeo.geo import Geometry


def _polyline(points, close=False) -> Geometry:
    geo = Geometry()
    geo.move_to(*points[0])
    for p in points[1:]:
        geo.line_to(*p)
    if close:
        geo.line_to(*points[0])
    return geo


def _concat(*geos: Geometry) -> Geometry:
    result = Geometry()
    for geo in geos:
        result.extend(geo)
    return result


def _contours(geo: Geometry) -> list[Geometry]:
    return geo.split_into_contours()


def _start(geo: Geometry) -> tuple[float, float]:
    first = next(iter(geo.iter_typed_commands()))
    return first.end[0], first.end[1]


def _end(geo: Geometry) -> tuple[float, float]:
    x, y, _z = geo.get_last_point()
    return x, y


def _dist(a, b) -> float:
    return math.hypot(a[0] - b[0], a[1] - b[1])


SQUARE = [(0, 0), (10, 0), (10, 10), (0, 10)]


class TestCloseOpenContours:
    def test_closes_contour_within_tolerance(self):
        geo = _polyline(SQUARE + [(0, 0.05)])
        result, count = geo.close_open_contours(0.1)
        assert count == 1
        contours = _contours(result)
        assert len(contours) == 1
        assert contours[0].is_closed()

    def test_leaves_contour_outside_tolerance_open(self):
        geo = _polyline(SQUARE + [(0, 0.5)])
        result, count = geo.close_open_contours(0.1)
        assert count == 0
        assert not _contours(result)[0].is_closed()

    def test_does_not_move_existing_points(self):
        geo = _polyline(SQUARE + [(0, 0.05)])
        result, _count = geo.close_open_contours(0.1)
        assert result.rect() == pytest.approx(geo.rect())

    def test_already_closed_contour_is_untouched(self):
        geo = _polyline(SQUARE, close=True)
        result, count = geo.close_open_contours(0.1)
        assert count == 0
        assert len(result.data) == len(geo.data)

    def test_handles_each_contour_separately(self):
        geo = _concat(
            _polyline(SQUARE + [(0, 0.05)]),
            _polyline([(20, 0), (30, 0), (30, 10), (20, 3)]),
            _polyline([(40, 0), (50, 0), (50, 10), (40, 0.02)]),
        )
        result, count = geo.close_open_contours(0.1)
        assert count == 2
        closed = [c.is_closed() for c in _contours(result)]
        assert closed == [True, False, True]

    def test_ignores_degenerate_single_segment(self):
        geo = _polyline([(0, 0), (0.05, 0)])
        _result, count = geo.close_open_contours(0.1)
        assert count == 0

    def test_empty_geometry(self):
        result, count = Geometry().close_open_contours(0.1)
        assert count == 0
        assert result.is_empty()


class TestJoinOpenContours:
    def test_joins_two_pieces_in_order(self):
        geo = _concat(
            _polyline([(0, 0), (10, 0), (10, 10)]),
            _polyline([(10, 10.05), (0, 10), (0, 0.05)]),
        )
        result, joins = geo.join_open_contours(0.1)
        assert joins == 1
        contours = _contours(result)
        assert len(contours) == 1
        assert contours[0].is_closed()

    def test_joins_reversed_piece(self):
        geo = _concat(
            _polyline([(0, 0), (10, 0)]),
            _polyline([(20, 0), (10.02, 0)]),
        )
        result, joins = geo.join_open_contours(0.1)
        assert joins == 1
        contours = _contours(result)
        assert len(contours) == 1
        ends = {_start(contours[0]), _end(contours[0])}
        assert ends == {(0.0, 0.0), (20.0, 0.0)}

    def test_joins_pieces_out_of_order(self):
        geo = _concat(
            _polyline([(0, 0), (10, 0)]),
            _polyline([(20, 0), (30, 0)]),
            _polyline([(10, 0), (20, 0)]),
        )
        result, joins = geo.join_open_contours(0.1)
        assert joins == 2
        contours = _contours(result)
        assert len(contours) == 1
        assert result.distance() == pytest.approx(30.0)

    def test_joins_at_the_start_of_a_piece(self):
        geo = _concat(
            _polyline([(10, 0), (20, 0)]),
            _polyline([(0, 0), (10, 0)]),
        )
        result, joins = geo.join_open_contours(0.1)
        assert joins == 1
        contours = _contours(result)
        assert len(contours) == 1
        assert {_start(contours[0]), _end(contours[0])} == {
            (0.0, 0.0),
            (20.0, 0.0),
        }

    def test_keeps_pieces_apart_beyond_tolerance(self):
        geo = _concat(
            _polyline([(0, 0), (10, 0)]),
            _polyline([(11, 0), (20, 0)]),
        )
        result, joins = geo.join_open_contours(0.1)
        assert joins == 0
        assert len(_contours(result)) == 2

    def test_leaves_closed_contours_alone(self):
        square = _polyline(SQUARE, close=True)
        line = _polyline([(0, 0), (-10, 0)])
        geo = _concat(square, line)
        result, joins = geo.join_open_contours(0.1)
        assert joins == 0
        assert len(_contours(result)) == 2

    def test_closes_single_piece_whose_ends_meet(self):
        geo = _polyline(SQUARE + [(0, 0.05)])
        result, _joins = geo.join_open_contours(0.1)
        assert _contours(result)[0].is_closed()

    def test_bridge_does_not_change_bounds(self):
        geo = _concat(
            _polyline([(0, 0), (10, 0), (10, 10)]),
            _polyline([(10.05, 10), (0, 10), (0, 0)]),
        )
        result, _joins = geo.join_open_contours(0.1)
        assert result.rect() == pytest.approx(geo.rect())

    def test_leaves_junction_with_several_ends_open(self):
        """Four rays meet at the origin, so no continuation is
        unambiguous. Chaining them used to trace one path that crosses
        itself; its windings cancel, leaving zero area, which downstream
        offsetting and inner/outer classification drop.
        """
        geo = _concat(
            _polyline([(0, 0), (10, 0)]),
            _polyline([(0, 0), (0, 10)]),
            _polyline([(0, 0), (-10, 0)]),
            _polyline([(0, 0), (0, -10)]),
        )
        result, joins = geo.join_open_contours(0.1)
        assert joins == 0
        assert len(_contours(result)) == 4

    def test_many_segments_form_one_closed_outline(self):
        pts = [
            (10 * math.cos(a), 10 * math.sin(a))
            for a in (2 * math.pi * i / 60 for i in range(60))
        ]
        pieces = []
        for i in range(60):
            a, b = pts[i], pts[(i + 1) % 60]
            if i % 2:
                a, b = b, a
            pieces.append(_polyline([a, b]))
        geo = _concat(*reversed(pieces))
        result, joins = geo.join_open_contours(0.01)
        assert joins == 59
        contours = _contours(result)
        assert len(contours) == 1
        assert contours[0].is_closed()


class TestRemoveDuplicateContours:
    def test_removes_identical_copy(self):
        geo = _concat(
            _polyline(SQUARE, close=True), _polyline(SQUARE, close=True)
        )
        result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 1
        assert len(_contours(result)) == 1

    def test_removes_reversed_copy(self):
        geo = _concat(
            _polyline(SQUARE, close=True),
            _polyline(list(reversed(SQUARE)), close=True),
        )
        _result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 1

    def test_removes_copy_with_other_start_point(self):
        shifted = SQUARE[2:] + SQUARE[:2]
        geo = _concat(
            _polyline(SQUARE, close=True), _polyline(shifted, close=True)
        )
        _result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 1

    def test_removes_open_reversed_copy(self):
        line = [(0, 0), (5, 5), (10, 0)]
        geo = _concat(_polyline(line), _polyline(list(reversed(line))))
        _result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 1

    def test_keeps_shapes_that_only_overlap_partly(self):
        geo = _concat(
            _polyline(SQUARE, close=True),
            _polyline([(0, 0), (10, 0), (10, 10)]),
        )
        result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 0
        assert len(_contours(result)) == 2

    def test_keeps_nested_shapes(self):
        inner = [(2, 2), (8, 2), (8, 8), (2, 8)]
        geo = _concat(
            _polyline(SQUARE, close=True), _polyline(inner, close=True)
        )
        _result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 0

    def test_tolerance_applies(self):
        nudged = [(x + 0.005, y) for x, y in SQUARE]
        geo = _concat(
            _polyline(SQUARE, close=True), _polyline(nudged, close=True)
        )
        assert geo.remove_duplicate_contours(0.01)[1] == 1
        assert geo.remove_duplicate_contours(0.001)[1] == 0

    def test_keeps_first_occurrence_and_order(self):
        other = [(20, 0), (30, 0), (30, 10)]
        geo = _concat(
            _polyline(SQUARE, close=True),
            _polyline(other),
            _polyline(SQUARE, close=True),
        )
        result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 1
        contours = _contours(result)
        assert _start(contours[0]) == (0.0, 0.0)
        assert _start(contours[1]) == (20.0, 0.0)

    def test_removes_triple_copies(self):
        geo = _concat(*(_polyline(SQUARE, close=True) for _ in range(3)))
        _result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 2

    def test_many_scattered_duplicates(self):
        pieces = []
        for i in range(200):
            x, y = (i * 37) % 290, (i * 53) % 290
            pieces.append(_polyline([(x, y), (x + 5, y)]))
        copies = [
            _polyline(list(reversed([(x, y), (x + 5, y)])))
            for x, y in (((i * 37) % 290, (i * 53) % 290) for i in range(200))
        ]
        geo = _concat(*pieces, *copies)
        result, removed = geo.remove_duplicate_contours(0.01)
        assert removed == 200
        assert len(_contours(result)) == 200


class TestGeometriesMatch:
    def test_same_shapes_in_other_order_match(self):
        a = _concat(
            _polyline(SQUARE, close=True), _polyline([(20, 0), (30, 0)])
        )
        b = _concat(
            _polyline([(30, 0), (20, 0)]), _polyline(SQUARE, close=True)
        )
        assert a.matches(b, 0.01)

    def test_different_shapes_do_not_match(self):
        a = _polyline(SQUARE, close=True)
        b = _polyline([(0, 0), (10, 0), (10, 11), (0, 10)], close=True)
        assert not a.matches(b, 0.01)

    def test_extra_contour_does_not_match(self):
        a = _polyline(SQUARE, close=True)
        b = _concat(
            _polyline(SQUARE, close=True), _polyline([(20, 0), (30, 0)])
        )
        assert not a.matches(b, 0.01)

    def test_empty_geometries_do_not_match(self):
        assert not Geometry().matches(Geometry(), 0.01)


class TestSplitDrawnContours:
    def test_splits_every_contour_including_holes_and_open_paths(self):
        inner = [(2, 2), (8, 2), (8, 8), (2, 8)]
        geo = _concat(
            _polyline(SQUARE, close=True),
            _polyline(inner, close=True),
            _polyline([(20, 0), (30, 0)]),
        )
        parts = geo.split_drawn_contours()
        assert len(parts) == 3
        assert parts[2].distance() == pytest.approx(10.0)

    def test_single_contour(self):
        assert len(_polyline(SQUARE, close=True).split_drawn_contours()) == 1

    def test_drops_move_only_contours(self):
        geo = _concat(_polyline(SQUARE, close=True), _polyline([(5, 5)]))
        parts = geo.split_drawn_contours()
        assert len(parts) == 1


def _loose_rectangle(gap=0.02) -> Geometry:
    """A rectangle as four separate segments whose ends do not meet.

    This is the shape of the Onshape exports in issue #668: each
    side is its own entity and no two corners coincide.
    """
    geo = Geometry()
    for start, end in [
        ((gap, 0), (100, 0)),
        ((100 + gap, gap), (100 + gap, 60)),
        ((100, 60 + gap), (0, 60 + gap)),
        ((-gap, 60), (-gap, 0)),
    ]:
        geo.move_to(*start)
        geo.line_to(*end)
    return geo


class TestRepairOutlines:
    """Joining, closing and de-duplicating in one pass."""

    def test_fragments_become_one_closed_contour(self):
        geometry = _loose_rectangle()
        assert len(geometry.split_into_contours()) == 4

        repaired, report = geometry.repair_outlines(0.05)

        contours = repaired.split_into_contours()
        assert len(contours) == 1
        assert contours[0].is_closed(1e-6)
        assert report.joins == 3
        assert report.open_remaining == 0

    def test_area_is_preserved(self):
        geometry = _loose_rectangle()

        repaired, _ = geometry.repair_outlines(0.05)

        assert repaired.area() == pytest.approx(100 * 60, rel=0.01)

    def test_report_is_not_unchanged(self):
        geometry = _loose_rectangle()

        _, report = geometry.repair_outlines(0.05)

        assert report.is_unchanged() is False

    def test_tolerance_below_the_gap_does_nothing(self):
        geometry = _loose_rectangle(gap=1.0)
        before = len(geometry.split_into_contours())

        repaired, report = geometry.repair_outlines(0.05)

        assert len(repaired.split_into_contours()) == before
        assert report.joins == 0
        assert report.closed == 0

    def test_larger_tolerance_closes_the_gap(self):
        geometry = _loose_rectangle(gap=1.0)

        repaired, report = geometry.repair_outlines(2.0)

        assert len(repaired.split_into_contours()) == 1
        assert report.joins > 0

    def test_closed_contours_are_left_alone(self):
        geometry = Geometry()
        geometry.move_to(0, 0)
        geometry.line_to(10, 0)
        geometry.line_to(10, 10)
        geometry.close_path()

        repaired, report = geometry.repair_outlines(0.1)

        assert report.closed == 0
        assert repaired.area() == pytest.approx(50.0)

    def test_duplicates_are_removed(self):
        geometry = Geometry()
        for _ in range(2):
            geometry.move_to(0, 0)
            geometry.line_to(10, 0)
            geometry.line_to(10, 10)
            geometry.close_path()

        repaired, report = geometry.repair_outlines(0.01)

        assert report.duplicates_removed == 1
        assert len(repaired.split_into_contours()) == 1

    def test_zero_tolerance_disables_repair(self):
        geometry = _loose_rectangle()

        repaired, report = geometry.repair_outlines(0.0)

        assert report.joins == 0
        assert report.closed == 0
        assert report.open_remaining == 4
        assert len(repaired.split_into_contours()) == 4

    def test_empty_geometry(self):
        repaired, report = Geometry().repair_outlines(0.1)

        assert repaired.is_empty()
        assert report.is_unchanged()

    def test_existing_points_are_not_moved(self):
        geometry = _loose_rectangle()
        before = {tuple(round(v, 6) for v in cmd.end) for cmd in geometry.data}

        repaired, _ = geometry.repair_outlines(0.05)
        after = {tuple(round(v, 6) for v in cmd.end) for cmd in repaired.data}

        assert before <= after

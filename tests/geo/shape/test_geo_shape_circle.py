import math

import pytest

from raygeo.geo.shape.circle import (
    does_circle_intersect_rect,
    find_tangent_circle_centers,
    get_circle_circle_intersections,
    get_line_circle_intersections,
    is_circle_inside_rect,
    line_segment_intersects_circle,
    nearest_tangent_circle_on_polyline,
    project_point_onto_circle,
)


@pytest.fixture
def selection_rect():
    return (10.0, 10.0, 50.0, 50.0)


def test_get_circle_circle_intersections():
    # Two intersection points
    c1, r1 = (0, 0), 5
    c2, r2 = (8, 0), 5
    intersections = get_circle_circle_intersections(c1, r1, c2, r2)
    assert len(intersections) == 2

    # Sort both actual and expected results for a stable, order-independent
    # comparison
    sorted_intersections = sorted(intersections)
    expected_intersections = sorted([(4.0, 3.0), (4.0, -3.0)])
    assert sorted_intersections == pytest.approx(expected_intersections)

    # One intersection point (tangent)
    c1, r1 = (0, 0), 5
    c2, r2 = (10, 0), 5
    intersections = get_circle_circle_intersections(c1, r1, c2, r2)
    assert len(intersections) == 1
    assert intersections[0] == pytest.approx((5, 0))

    # No intersection (separate)
    c1, r1 = (0, 0), 5
    c2, r2 = (11, 0), 5
    assert get_circle_circle_intersections(c1, r1, c2, r2) == []

    # No intersection (one inside other)
    c1, r1 = (0, 0), 10
    c2, r2 = (1, 0), 1
    assert get_circle_circle_intersections(c1, r1, c2, r2) == []

    # Coincident circles
    c1, r1 = (0, 0), 5
    c2, r2 = (0, 0), 5
    assert get_circle_circle_intersections(c1, r1, c2, r2) == []


def test_is_circle_inside_rect(selection_rect):
    # Fully contained
    assert is_circle_inside_rect((30, 30), 10, selection_rect)
    # Touching edge
    assert is_circle_inside_rect((20, 30), 10, selection_rect)
    # Intersecting
    assert not is_circle_inside_rect((5, 30), 10, selection_rect)
    # Outside
    assert not is_circle_inside_rect((100, 100), 5, selection_rect)


def test_does_circle_intersect_rect(selection_rect):
    # Intersects
    assert does_circle_intersect_rect((5, 30), 10, selection_rect)
    # Fully contained (should not intersect boundary)
    assert not does_circle_intersect_rect((30, 30), 5, selection_rect)
    # Rect is fully contained in circle (should not intersect boundary)
    assert not does_circle_intersect_rect((30, 30), 100, selection_rect)
    # Touching
    assert does_circle_intersect_rect((0, 30), 10, selection_rect)
    # Separate
    assert not does_circle_intersect_rect((100, 100), 5, selection_rect)


def test_project_point_onto_circle_basic():
    """Test projecting a point onto circle circumference."""
    center = (0, 0)
    radius = 10.0
    point = (20, 0)
    result = project_point_onto_circle(point, center, radius)
    assert result is not None
    assert result == pytest.approx((10.0, 0.0))


def test_project_point_onto_circle_quadrants():
    """Test projection in all four quadrants."""
    center = (0, 0)
    radius = 5.0

    # Quadrant I
    result = project_point_onto_circle((10, 10), center, radius)
    assert result is not None
    assert result[0] > 0 and result[1] > 0

    # Quadrant II
    result = project_point_onto_circle((-10, 10), center, radius)
    assert result is not None
    assert result[0] < 0 and result[1] > 0

    # Quadrant III
    result = project_point_onto_circle((-10, -10), center, radius)
    assert result is not None
    assert result[0] < 0 and result[1] < 0

    # Quadrant IV
    result = project_point_onto_circle((10, -10), center, radius)
    assert result is not None
    assert result[0] > 0 and result[1] < 0


def test_project_point_onto_circle_at_center():
    """Test projecting from center returns None."""
    center = (0, 0)
    radius = 10.0
    result = project_point_onto_circle((0, 0), center, radius)
    assert result is None


def test_project_point_onto_circle_near_center():
    """Test projecting from near center returns None."""
    center = (0, 0)
    radius = 10.0
    result = project_point_onto_circle((1e-10, 1e-10), center, radius)
    assert result is None


def test_project_point_onto_circle_on_circumference():
    """Test projecting a point already on the circle."""
    center = (0, 0)
    radius = 10.0
    point = (10, 0)
    result = project_point_onto_circle(point, center, radius)
    assert result == pytest.approx(point)


def test_project_point_onto_circle_offset_center():
    """Test projection with offset center."""
    center = (100, 200)
    radius = 50.0
    point = (150, 200)
    result = project_point_onto_circle(point, center, radius)
    assert result is not None
    assert result == pytest.approx((150.0, 200.0))


def test_project_point_onto_circle_diagonal():
    """Test projection from diagonal direction."""
    center = (0, 0)
    radius = math.sqrt(2)
    point = (10, 10)
    result = project_point_onto_circle(point, center, radius)
    assert result is not None
    # Projected point should be on the 45-degree line
    assert abs(result[0] - result[1]) < 1e-9
    # Distance from center should equal radius
    dist = math.hypot(result[0], result[1])
    assert dist == pytest.approx(radius)


class TestLineSegmentIntersectsCircle:
    def test_segment_crosses_circle(self):
        assert line_segment_intersects_circle((0, 0), (10, 0), (5, 0), 2)

    def test_segment_tangent_to_circle(self):
        assert line_segment_intersects_circle((0, 0), (10, 0), (5, 2), 2)

    def test_segment_outside_circle(self):
        assert not line_segment_intersects_circle((0, 5), (10, 5), (5, 0), 2)

    def test_segment_entirely_inside_circle(self):
        assert line_segment_intersects_circle((4, 0), (6, 0), (5, 0), 10)

    def test_one_endpoint_inside(self):
        assert line_segment_intersects_circle((4, 0), (20, 0), (5, 0), 3)

    def test_zero_length_inside(self):
        assert line_segment_intersects_circle((5, 0), (5, 0), (5, 0), 1)

    def test_zero_length_outside(self):
        assert not line_segment_intersects_circle((10, 0), (10, 0), (5, 0), 1)


class TestGetLineCircleIntersections:
    def test_two_intersections(self):
        results = get_line_circle_intersections((-2, 0), (2, 0), (0, 0), 1)
        assert len(results) == 2
        sorted_pts = sorted(results)
        assert sorted_pts == pytest.approx([(-1, 0), (1, 0)])

    def test_tangent(self):
        results = get_line_circle_intersections((0, 1), (2, 1), (1, 0), 1)
        assert len(results) == 1
        assert results[0] == pytest.approx((1, 1))

    def test_no_intersection(self):
        results = get_line_circle_intersections((0, 3), (2, 3), (1, 0), 1)
        assert results == []

    def test_segment_before_circle(self):
        results = get_line_circle_intersections((-5, 0), (-2, 0), (0, 0), 1)
        assert results == []

    def test_segment_after_circle(self):
        results = get_line_circle_intersections((2, 0), (5, 0), (0, 0), 1)
        assert results == []

    def test_one_endpoint_inside(self):
        results = get_line_circle_intersections((0, 0), (3, 0), (0, 0), 1)
        assert len(results) == 1
        assert results[0] == pytest.approx((1, 0))

    def test_diagonal_intersection(self):
        results = get_line_circle_intersections((-2, -2), (2, 2), (0, 0), 1)
        assert len(results) == 2
        expected_dist = 1.0 / math.sqrt(2)
        for px, py in results:
            assert math.hypot(px, py) == pytest.approx(1.0)
            assert abs(px) == pytest.approx(expected_dist)
            assert abs(py) == pytest.approx(expected_dist)

    def test_offset_center(self):
        results = get_line_circle_intersections((8, -2), (8, 2), (8, 0), 1)
        assert len(results) == 2
        sorted_pts = sorted(results, key=lambda p: p[1])
        assert sorted_pts == pytest.approx([(8, -1), (8, 1)])

    def test_zero_length_segment(self):
        results = get_line_circle_intersections((5, 5), (5, 5), (0, 0), 10)
        assert results == []

    def test_both_endpoints_on_circle(self):
        results = get_line_circle_intersections((-1, 0), (1, 0), (0, 0), 1)
        assert len(results) == 2
        sorted_pts = sorted(results)
        assert sorted_pts == pytest.approx([(-1, 0), (1, 0)])


def _dist(p, q):
    return math.hypot(p[0] - q[0], p[1] - q[1])


class TestFindTangentCircleCenters:
    def test_two_solutions_perpendicular(self):
        """Point perpendicular to segment midpoint → two circles."""
        results = find_tangent_circle_centers((5, 3), (0, 0), (10, 0), 2.0)
        assert len(results) == 2
        for center, tangent in results:
            assert _dist(center, (5, 3)) == pytest.approx(2.0)
            assert _dist(center, tangent) == pytest.approx(2.0)
            assert 0.0 <= tangent[0] <= 10.0
            assert tangent[1] == pytest.approx(0.0)

    def test_solutions_on_opposite_sides(self):
        """Point between the two offset lines yields one circle per side."""
        results = find_tangent_circle_centers((5, 3), (0, 0), (10, 0), 2.0)
        assert len(results) == 2
        for center, tangent in results:
            assert _dist(center, (5, 3)) == pytest.approx(2.0)
            assert _dist(center, tangent) == pytest.approx(2.0)
            assert tangent[1] == pytest.approx(0.0)
            assert 0.0 <= tangent[0] <= 10.0

    def test_four_results_point_on_segment(self):
        """Point on the segment gives 4 entries (2 per side, duplicated)."""
        results = find_tangent_circle_centers((5, 0), (0, 0), (10, 0), 3.0)
        assert len(results) == 4
        for center, tangent in results:
            assert _dist(center, (5, 0)) == pytest.approx(3.0)
            assert _dist(center, tangent) == pytest.approx(3.0)
            assert tangent == (5.0, 0.0)

    def test_one_solution_tangent(self):
        """Point at distance exactly 2r → one circle per side (tangent)."""
        results = find_tangent_circle_centers((5, 10), (0, 0), (10, 0), 5.0)
        assert len(results) == 2
        for center, tangent in results:
            assert _dist(center, (5, 10)) == pytest.approx(5.0)
            assert _dist(center, tangent) == pytest.approx(5.0)

    def test_no_solution_radius_too_small(self):
        """Point too far from line to be reached with given radius."""
        results = find_tangent_circle_centers((5, 20), (0, 0), (10, 0), 5.0)
        assert results == []

    def test_point_at_segment_endpoint(self):
        """Point at seg_a yields 4 entries (2 per side, duplicated)."""
        results = find_tangent_circle_centers((0, 0), (0, 0), (10, 0), 4.0)
        assert len(results) == 4
        for center, tangent in results:
            assert _dist(center, (0, 0)) == pytest.approx(4.0)
            assert _dist(center, tangent) == pytest.approx(4.0)
            assert 0.0 <= tangent[0] <= 10.0
            assert tangent[1] == pytest.approx(0.0)

    def test_no_solution_beyond_endpoint(self):
        """Tangent point would fall outside the segment → filtered out."""
        results = find_tangent_circle_centers((-10, 10), (0, 0), (10, 0), 5.0)
        # All candidate tangent points likely fall outside [0, 10]
        assert results == []

    def test_vertical_segment(self):
        """Vertical segment still works."""
        results = find_tangent_circle_centers((3, 5), (0, 0), (0, 10), 2.0)
        assert len(results) == 2
        for center, tangent in results:
            assert _dist(center, (3, 5)) == pytest.approx(2.0)
            assert _dist(center, tangent) == pytest.approx(2.0)
            assert tangent[0] == pytest.approx(0.0)
            assert 0.0 <= tangent[1] <= 10.0

    def test_zero_length_segment(self):
        """Zero-length segment returns empty."""
        assert find_tangent_circle_centers((0, 0), (5, 0), (5, 0), 2.0) == []

    def test_non_positive_radius(self):
        """Zero or negative radius returns empty."""
        assert find_tangent_circle_centers((0, 0), (0, 0), (10, 0), 0.0) == []
        assert find_tangent_circle_centers((0, 0), (0, 0), (10, 0), -1.0) == []
        assert (
            find_tangent_circle_centers((0, 0), (0, 0), (10, 0), -0.001) == []
        )

    def test_all_results_unique(self):
        """All returned (center, tangent) pairs should be distinct."""
        results = find_tangent_circle_centers((5, 6), (0, 0), (10, 0), 4.0)
        pairs = [
            (
                round(c[0], 10),
                round(c[1], 10),
                round(t[0], 10),
                round(t[1], 10),
            )
            for c, t in results
        ]
        assert len(set(pairs)) == len(pairs)


def _square_containment(x, y, size):
    """Helper: a square CCW polygon centered at (x, y) with side `size`."""
    h = size / 2.0
    return [
        (float(x - h), float(y - h)),
        (float(x + h), float(y - h)),
        (float(x + h), float(y + h)),
        (float(x - h), float(y + h)),
    ]


class TestNearestTangentCircleOnPolyline:
    def test_forward_on_l_shape(self):
        """L-shape polyline, forward search → nearest to first vertex."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        point = (5, 8)
        radius = 5.0
        containment = _square_containment(5, 5, 20)
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=False,
            containment=containment,
        )
        assert result is not None
        center, tangent, idx = result
        assert idx == 0
        assert tangent == pytest.approx((1, 0))
        assert center == pytest.approx((1, 5))

    def test_from_end_on_l_shape(self):
        """L-shape polyline, search from end → nearest to last vertex."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        point = (5, 8)
        radius = 5.0
        containment = _square_containment(5, 5, 20)
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=True,
            containment=containment,
        )
        assert result is not None
        center, tangent, idx = result
        assert idx == 1
        assert tangent == pytest.approx((10, 3))
        assert center == pytest.approx((5, 3))

    def test_containment_filters_all(self):
        """Tight containment that excludes all candidate centers → None."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        point = (5, 8)
        radius = 5.0
        # Tiny box far from any candidate center
        containment = _square_containment(100, 100, 1)
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=False,
            containment=containment,
        )
        assert result is None

    def test_containment_selects_specific_candidate(self):
        """Tight containment selects only the candidate inside it."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        point = (5, 5)
        radius = 3.0
        # Candidates from (0,0)-(10,0) with r=3:
        # offset y=3: disc = -2*3*5 - 25 = -55 < 0
        # Actually: side=1.0 offset_origin=(0,3), f=(-5,-2), fd=-5
        # disc = 25 - (25+4) + 9 = 5 > 0, sq=√5≈2.236
        # t = 5±2.236 = 2.764, 7.236
        # centers: (2.764, 3), (7.236, 3)
        # Both have y=3, so containment at y in [2,4] contains both.
        # Let's put containment around only the first candidate:
        containment = _square_containment(2.764, 3, 2)
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=False,
            containment=containment,
        )
        assert result is not None
        center, tangent, idx = result
        assert idx == 0
        assert center == pytest.approx((2.7639320225002102, 3.0))

    def test_no_valid_circle_radius_too_small(self):
        """Radius too small to reach the point from any segment → None."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        point = (5, 100)
        radius = 5.0
        containment = _square_containment(0, 0, 200)
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=False,
            containment=containment,
        )
        assert result is None

    def test_single_segment_polyline(self):
        """A 2-point (single segment) polyline works."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        point = (5, 4)
        radius = 3.0
        containment = _square_containment(5, 3, 10)
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=False,
            containment=containment,
        )
        assert result is not None
        center, tangent, idx = result
        assert idx == 0
        assert _dist(center, (5, 4)) == pytest.approx(3.0)
        assert _dist(center, tangent) == pytest.approx(3.0)
        assert 0.0 <= tangent[0] <= 10.0

    def test_less_than_2_points(self):
        """Polyline with < 2 points returns None."""
        assert (
            nearest_tangent_circle_on_polyline(
                (0, 0), [(5, 5)], 2.0, False, _square_containment(0, 0, 20)
            )
            is None
        )
        assert (
            nearest_tangent_circle_on_polyline(
                (0, 0), [], 2.0, False, _square_containment(0, 0, 20)
            )
            is None
        )

    def test_non_positive_radius(self):
        """Zero or negative radius returns None."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        containment = _square_containment(5, 0, 20)
        assert (
            nearest_tangent_circle_on_polyline(
                (5, 3), polyline, 0.0, False, containment
            )
            is None
        )
        assert (
            nearest_tangent_circle_on_polyline(
                (5, 3), polyline, -1.0, False, containment
            )
            is None
        )

    def test_from_end_single_segment(self):
        """Single-segment polyline with from_end=True still works."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        point = (5, 4)
        radius = 3.0
        containment = _square_containment(5, 0, 20)
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=True,
            containment=containment,
        )
        assert result is not None
        center, tangent, idx = result
        assert idx == 0

    def test_early_break_forward(self):
        """Early-break fires when later segments can't beat best (forward)."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)]
        point = (1, 4)
        radius = 3.0
        containment = _square_containment(4, 3, 10)
        # Segment 0: (0,0)→(10,0), tangent at x≈3.828
        # dist_sq to reference (0,0) ≈ 14.7, seg_ref = polyline[1] = (10,0)
        # dist_sq((10,0),(0,0)) = 100 > 14.7+9 → early break
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=False,
            containment=containment,
        )
        assert result is not None
        assert result[2] == 0

    def test_early_break_from_end(self):
        """Early-break fires when later segments can't beat best (from end)."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)]
        point = (19, 4)
        radius = 3.0
        containment = _square_containment(16, 3, 10)
        # Segment 1: (10,0)→(20,0), tangent at x≈16.172
        # dist_sq to reference (20,0) ≈ 14.7, seg_ref=polyline[1]=(10,0)
        # dist_sq((10,0),(20,0)) = 100 > 14.7+9 → early break
        result = nearest_tangent_circle_on_polyline(
            point,
            polyline,
            radius,
            from_end=True,
            containment=containment,
        )
        assert result is not None
        assert result[2] == 1

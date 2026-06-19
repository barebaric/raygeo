"""
Tests for raygeo.polygon module.
"""

from typing import List, cast

import numpy as np
import pytest

from raygeo.geo.algo.smooth import resample_polyline
from raygeo.geo.shape.polygon import (
    apply_minimum_curvature,
    clean_polygon,
    flip_polygon_numpy,
    flip_polygons_numpy,
    get_polygon_area,
    get_polygon_bounds,
    get_polygon_centroid,
    get_polygon_closest_point,
    get_polygon_convex_hull,
    get_polygon_edges,
    get_polygon_group_bounds,
    get_polygon_perimeter,
    get_polygon_signed_area,
    get_polygons_difference,
    get_polygons_group_difference,
    get_polygons_group_intersection,
    get_polygons_intersection,
    get_polygons_union,
    is_almost_equal,
    is_point_inside_polygon,
    is_polygon_convex,
    normalize_polygons,
    normalize_polygons_numpy,
    offset_polygon,
    point_in_polygon_numpy,
    point_line_distance,
    polygon_area_numpy,
    polygon_bounds_numpy,
    polygon_group_bounds_numpy,
    polygon_perimeter_numpy,
    polygons_intersect,
    polygons_intersect_numpy,
    rotate_polygon,
    rotate_polygon_numpy,
    rotate_polygons,
    rotate_polygons_numpy,
    scale_polygon,
    translate_bounds,
    translate_polygon,
    translate_polygon_numpy,
    translate_polygons,
    translate_polygons_numpy,
)
from raygeo.geo.types import Polygon


def P(*points) -> Polygon:
    """Helper to create a polygon from integer points."""
    return [(float(x), float(y)) for x, y in points]


def PN(*points) -> np.ndarray:
    """Helper to create a numpy polygon from integer points."""
    return np.array([[float(x), float(y)] for x, y in points], dtype=float)


class TestPolygonArea:
    def test_triangle(self):
        polygon = P((0, 0), (10, 0), (5, 5))
        area = get_polygon_area(polygon)
        assert abs(area - 25.0) < 0.001

    def test_square(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        area = get_polygon_area(polygon)
        assert abs(area - 100.0) < 0.001

    def test_ccw_positive(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        area = get_polygon_area(polygon)
        assert area > 0

    def test_cw_negative(self):
        polygon = P((0, 0), (0, 10), (10, 10), (10, 0))
        area = get_polygon_area(polygon)
        assert area < 0

    def test_empty(self):
        assert get_polygon_area(cast(Polygon, [])) == 0.0

    def test_single_point(self):
        assert get_polygon_area(P((0, 0))) == 0.0

    def test_two_points(self):
        assert get_polygon_area(P((0, 0), (1, 1))) == 0.0


class TestPolygonAreaNumpy:
    def test_triangle(self):
        polygon = PN((0, 0), (10, 0), (5, 5))
        area = polygon_area_numpy(polygon)
        assert abs(area - 25.0) < 0.001

    def test_square(self):
        polygon = PN((0, 0), (10, 0), (10, 10), (0, 10))
        area = polygon_area_numpy(polygon)
        assert abs(area - 100.0) < 0.001

    def test_ccw_positive(self):
        polygon = PN((0, 0), (10, 0), (10, 10), (0, 10))
        area = polygon_area_numpy(polygon)
        assert area > 0

    def test_cw_negative(self):
        polygon = PN((0, 0), (0, 10), (10, 10), (10, 0))
        area = polygon_area_numpy(polygon)
        assert area < 0

    def test_empty(self):
        assert polygon_area_numpy(np.array([]).reshape(0, 2)) == 0.0

    def test_single_point(self):
        assert polygon_area_numpy(PN((0, 0))) == 0.0

    def test_two_points(self):
        assert polygon_area_numpy(PN((0, 0), (1, 1))) == 0.0


class TestPolygonBounds:
    def test_basic(self):
        polygon = P((1, 2), (5, 3), (3, 7), (0, 5))
        min_x, min_y, max_x, max_y = get_polygon_bounds(polygon)
        assert min_x == 0
        assert min_y == 2
        assert max_x == 5
        assert max_y == 7

    def test_empty(self):
        assert get_polygon_bounds(cast(Polygon, [])) == (0.0, 0.0, 0.0, 0.0)

    def test_single_point(self):
        min_x, min_y, max_x, max_y = get_polygon_bounds(P((5, 10)))
        assert min_x == max_x == 5
        assert min_y == max_y == 10


class TestPolygonBoundsNumpy:
    def test_basic(self):
        polygon = PN((1, 2), (5, 3), (3, 7), (0, 5))
        min_x, min_y, max_x, max_y = polygon_bounds_numpy(polygon)
        assert min_x == 0
        assert min_y == 2
        assert max_x == 5
        assert max_y == 7

    def test_empty(self):
        assert polygon_bounds_numpy(np.array([]).reshape(0, 2)) == (
            0.0,
            0.0,
            0.0,
            0.0,
        )

    def test_single_point(self):
        min_x, min_y, max_x, max_y = polygon_bounds_numpy(PN((5, 10)))
        assert min_x == max_x == 5
        assert min_y == max_y == 10


class TestGroupBounds:
    def test_multiple_polygons(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
        min_x, min_y, max_x, max_y = get_polygon_group_bounds([poly1, poly2])
        assert min_x == 0
        assert min_y == 0
        assert max_x == 15
        assert max_y == 15

    def test_single_polygon(self):
        polygon = P((1, 2), (5, 3), (3, 7), (0, 5))
        min_x, min_y, max_x, max_y = get_polygon_group_bounds([polygon])
        assert min_x == 0
        assert min_y == 2
        assert max_x == 5
        assert max_y == 7

    def test_empty_list(self):
        assert get_polygon_group_bounds([]) == (0.0, 0.0, 0.0, 0.0)

    def test_list_with_empty_polygons(self):
        assert get_polygon_group_bounds(
            [cast(Polygon, []), cast(Polygon, [])]
        ) == (
            0.0,
            0.0,
            0.0,
            0.0,
        )

    def test_disjoint_polygons(self):
        poly1 = P((0, 0), (1, 0), (1, 1), (0, 1))
        poly2 = P((10, 10), (20, 10), (20, 20), (10, 20))
        min_x, min_y, max_x, max_y = get_polygon_group_bounds([poly1, poly2])
        assert min_x == 0
        assert min_y == 0
        assert max_x == 20
        assert max_y == 20


class TestGroupBoundsNumpy:
    def test_multiple_polygons(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((5, 5), (15, 5), (15, 15), (5, 15))
        min_x, min_y, max_x, max_y = polygon_group_bounds_numpy([poly1, poly2])
        assert min_x == 0
        assert min_y == 0
        assert max_x == 15
        assert max_y == 15

    def test_single_polygon(self):
        polygon = PN((1, 2), (5, 3), (3, 7), (0, 5))
        min_x, min_y, max_x, max_y = polygon_group_bounds_numpy([polygon])
        assert min_x == 0
        assert min_y == 2
        assert max_x == 5
        assert max_y == 7

    def test_empty_list(self):
        assert polygon_group_bounds_numpy([]) == (0.0, 0.0, 0.0, 0.0)

    def test_list_with_empty_polygons(self):
        assert polygon_group_bounds_numpy(
            [np.array([]).reshape(0, 2), np.array([]).reshape(0, 2)]
        ) == (
            0.0,
            0.0,
            0.0,
            0.0,
        )

    def test_disjoint_polygons(self):
        poly1 = PN((0, 0), (1, 0), (1, 1), (0, 1))
        poly2 = PN((10, 10), (20, 10), (20, 20), (10, 20))
        min_x, min_y, max_x, max_y = polygon_group_bounds_numpy([poly1, poly2])
        assert min_x == 0
        assert min_y == 0
        assert max_x == 20
        assert max_y == 20


class TestPolygonCentroid:
    def test_square(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        cx, cy = get_polygon_centroid(polygon)
        assert is_almost_equal(cx, 5.0)
        assert is_almost_equal(cy, 5.0)

    def test_triangle(self):
        polygon = P((0, 0), (6, 0), (3, 6))
        cx, cy = get_polygon_centroid(polygon)
        assert is_almost_equal(cx, 3.0)
        assert is_almost_equal(cy, 2.0)

    def test_empty(self):
        assert get_polygon_centroid(cast(Polygon, [])) == (0.0, 0.0)


class TestRotatePolygon:
    def test_90_degrees(self):
        polygon = P((1, 0), (2, 0), (2, 1))
        rotated = rotate_polygon(polygon, 90)
        assert is_almost_equal(rotated[0][0], 0)
        assert is_almost_equal(rotated[0][1], 1)

    def test_180_degrees(self):
        polygon = P((1, 0), (2, 0), (2, 1))
        rotated = rotate_polygon(polygon, 180)
        assert is_almost_equal(rotated[0][0], -1)
        assert is_almost_equal(rotated[0][1], 0)

    def test_360_degrees(self):
        polygon = P((1, 0), (2, 0), (2, 1))
        rotated = rotate_polygon(polygon, 360)
        assert is_almost_equal(rotated[0][0], 1)
        assert is_almost_equal(rotated[0][1], 0)

    def test_0_degrees(self):
        polygon = P((1, 2), (3, 4))
        rotated = rotate_polygon(polygon, 0)
        assert rotated == polygon

    def test_negative_angle(self):
        polygon = P((1, 0), (2, 0), (2, 1))
        rotated = rotate_polygon(polygon, -90)
        assert is_almost_equal(rotated[0][0], 0)
        assert is_almost_equal(rotated[0][1], -1)


class TestRotatePolygonNumpy:
    def test_90_degrees(self):
        polygon = PN((1, 0), (2, 0), (2, 1))
        rotated = rotate_polygon_numpy(polygon, 90)
        assert is_almost_equal(rotated[0, 0], 0)
        assert is_almost_equal(rotated[0, 1], 1)

    def test_180_degrees(self):
        polygon = PN((1, 0), (2, 0), (2, 1))
        rotated = rotate_polygon_numpy(polygon, 180)
        assert is_almost_equal(rotated[0, 0], -1)
        assert is_almost_equal(rotated[0, 1], 0)

    def test_360_degrees(self):
        polygon = PN((1, 0), (2, 0), (2, 1))
        rotated = rotate_polygon_numpy(polygon, 360)
        assert is_almost_equal(rotated[0, 0], 1)
        assert is_almost_equal(rotated[0, 1], 0)

    def test_0_degrees(self):
        polygon = PN((1, 2), (3, 4))
        rotated = rotate_polygon_numpy(polygon, 0)
        np.testing.assert_array_almost_equal(rotated, polygon)

    def test_negative_angle(self):
        polygon = PN((1, 0), (2, 0), (2, 1))
        rotated = rotate_polygon_numpy(polygon, -90)
        assert is_almost_equal(rotated[0, 0], 0)
        assert is_almost_equal(rotated[0, 1], -1)


class TestTranslatePolygon:
    def test_basic(self):
        polygon = P((0, 0), (10, 0), (5, 5))
        translated = translate_polygon(polygon, 5, 10)
        assert translated[0] == (5.0, 10.0)
        assert translated[1] == (15.0, 10.0)
        assert translated[2] == (10.0, 15.0)

    def test_negative(self):
        polygon = P((10, 20), (30, 40))
        translated = translate_polygon(polygon, -5, -10)
        assert translated[0] == (5.0, 10.0)
        assert translated[1] == (25.0, 30.0)

    def test_zero(self):
        polygon = P((1, 2), (3, 4))
        translated = translate_polygon(polygon, 0, 0)
        assert translated == polygon


class TestTranslatePolygonNumpy:
    def test_basic(self):
        polygon = PN((0, 0), (10, 0), (5, 5))
        translated = translate_polygon_numpy(polygon, 5, 10)
        assert translated[0, 0] == 5.0
        assert translated[0, 1] == 10.0
        assert translated[1, 0] == 15.0
        assert translated[1, 1] == 10.0
        assert translated[2, 0] == 10.0
        assert translated[2, 1] == 15.0

    def test_negative(self):
        polygon = PN((10, 20), (30, 40))
        translated = translate_polygon_numpy(polygon, -5, -10)
        assert translated[0, 0] == 5.0
        assert translated[0, 1] == 10.0
        assert translated[1, 0] == 25.0
        assert translated[1, 1] == 30.0

    def test_zero(self):
        polygon = PN((1, 2), (3, 4))
        translated = translate_polygon_numpy(polygon, 0, 0)
        np.testing.assert_array_almost_equal(translated, polygon)


class TestRotatePolygons:
    def test_multiple_polygons(self):
        poly1 = P((1, 0), (2, 0), (2, 1))
        poly2 = P((3, 0), (4, 0), (4, 1))
        rotated = rotate_polygons([poly1, poly2], 90)
        assert len(rotated) == 2
        assert is_almost_equal(rotated[0][0][0], 0)
        assert is_almost_equal(rotated[0][0][1], 1)
        assert is_almost_equal(rotated[1][0][0], 0)
        assert is_almost_equal(rotated[1][0][1], 3)

    def test_empty_list(self):
        rotated = rotate_polygons([], 90)
        assert rotated == []

    def test_preserves_count(self):
        polygons = [P((1, 0), (2, 0)), P((3, 0), (4, 0)), P((5, 0), (6, 0))]
        rotated = rotate_polygons(polygons, 45)
        assert len(rotated) == 3


class TestRotatePolygonsNumpy:
    def test_multiple_polygons(self):
        poly1 = PN((1, 0), (2, 0), (2, 1))
        poly2 = PN((3, 0), (4, 0), (4, 1))
        rotated = rotate_polygons_numpy([poly1, poly2], 90)
        assert len(rotated) == 2
        assert is_almost_equal(rotated[0][0, 0], 0)
        assert is_almost_equal(rotated[0][0, 1], 1)
        assert is_almost_equal(rotated[1][0, 0], 0)
        assert is_almost_equal(rotated[1][0, 1], 3)

    def test_empty_list(self):
        rotated = rotate_polygons_numpy([], 90)
        assert rotated == []

    def test_preserves_count(self):
        polygons = [PN((1, 0), (2, 0)), PN((3, 0), (4, 0)), PN((5, 0), (6, 0))]
        rotated = rotate_polygons_numpy(polygons, 45)
        assert len(rotated) == 3


class TestTranslatePolygons:
    def test_multiple_polygons(self):
        poly1 = P((0, 0), (10, 0), (5, 5))
        poly2 = P((20, 20), (30, 20), (25, 25))
        translated = translate_polygons([poly1, poly2], 5, 10)
        assert len(translated) == 2
        assert translated[0][0] == (5.0, 10.0)
        assert translated[1][0] == (25.0, 30.0)

    def test_negative(self):
        poly1 = P((10, 20), (30, 40))
        poly2 = P((50, 60), (70, 80))
        translated = translate_polygons([poly1, poly2], -5, -10)
        assert translated[0][0] == (5.0, 10.0)
        assert translated[1][0] == (45.0, 50.0)

    def test_zero(self):
        polygons = [P((1, 2), (3, 4)), P((5, 6), (7, 8))]
        translated = translate_polygons(polygons, 0, 0)
        assert translated == polygons

    def test_empty_list(self):
        translated = translate_polygons([], 5, 10)
        assert translated == []


class TestTranslatePolygonsNumpy:
    def test_multiple_polygons(self):
        poly1 = PN((0, 0), (10, 0), (5, 5))
        poly2 = PN((20, 20), (30, 20), (25, 25))
        translated = translate_polygons_numpy([poly1, poly2], 5, 10)
        assert len(translated) == 2
        assert translated[0][0, 0] == 5.0
        assert translated[0][0, 1] == 10.0
        assert translated[1][0, 0] == 25.0
        assert translated[1][0, 1] == 30.0

    def test_negative(self):
        poly1 = PN((10, 20), (30, 40))
        poly2 = PN((50, 60), (70, 80))
        translated = translate_polygons_numpy([poly1, poly2], -5, -10)
        assert translated[0][0, 0] == 5.0
        assert translated[0][0, 1] == 10.0
        assert translated[1][0, 0] == 45.0
        assert translated[1][0, 1] == 50.0

    def test_zero(self):
        polygons = [PN((1, 2), (3, 4)), PN((5, 6), (7, 8))]
        translated = translate_polygons_numpy(polygons, 0, 0)
        for i in range(len(polygons)):
            np.testing.assert_array_almost_equal(translated[i], polygons[i])

    def test_empty_list(self):
        translated = translate_polygons_numpy([], 5, 10)
        assert translated == []


class TestScalePolygon:
    def test_uniform(self):
        polygon = P((1, 1), (3, 1), (3, 3), (1, 3))
        scaled = scale_polygon(polygon, 2)
        assert scaled[0] == (2.0, 2.0)
        assert scaled[2] == (6.0, 6.0)

    def test_non_uniform(self):
        polygon = P((1, 1), (3, 1), (3, 3), (1, 3))
        scaled = scale_polygon(polygon, 2, 3)
        assert scaled[0] == (2.0, 3.0)
        assert scaled[2] == (6.0, 9.0)

    def test_shrink(self):
        polygon = P((2, 2), (6, 2), (6, 6), (2, 6))
        scaled = scale_polygon(polygon, 0.5)
        assert scaled[0] == (1.0, 1.0)
        assert scaled[2] == (3.0, 3.0)


class TestConvexHull:
    def test_basic(self):
        polygon = P((0, 0), (5, 3), (10, 0), (5, 5), (5, 2))
        hull = get_polygon_convex_hull(polygon)
        assert len(hull) >= 3
        hull_set = set(hull)
        assert (0.0, 0.0) in hull_set
        assert (10.0, 0.0) in hull_set
        assert (5.0, 5.0) in hull_set

    def test_already_convex(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        hull = get_polygon_convex_hull(polygon)
        assert len(hull) == 4

    def test_triangle(self):
        polygon = P((0, 0), (5, 10), (10, 0))
        hull = get_polygon_convex_hull(polygon)
        assert len(hull) == 3


class TestIsConvex:
    def test_square(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        assert is_polygon_convex(polygon) is True

    def test_triangle(self):
        polygon = P((0, 0), (5, 10), (10, 0))
        assert is_polygon_convex(polygon) is True

    def test_pentagon(self):
        polygon = P((0, 0), (5, -2), (10, 0), (8, 8), (2, 8))
        assert is_polygon_convex(polygon) is True

    def test_concave_quadrilateral(self):
        polygon = P((0, 0), (10, 0), (5, 5), (10, 10), (0, 10))
        assert is_polygon_convex(polygon) is False

    def test_arrow_shape(self):
        polygon = P((0, 0), (5, 5), (10, 0), (5, 10))
        assert is_polygon_convex(polygon) is False

    def test_empty(self):
        assert is_polygon_convex(cast(Polygon, [])) is False

    def test_single_point(self):
        assert is_polygon_convex(P((0, 0))) is False

    def test_two_points(self):
        assert is_polygon_convex(P((0, 0), (1, 1))) is False

    def test_collinear_points(self):
        polygon = P((0, 0), (5, 0), (10, 0), (5, 5))
        assert is_polygon_convex(polygon) is True

    def test_clockwise_square(self):
        polygon = P((0, 0), (0, 10), (10, 10), (10, 0))
        assert is_polygon_convex(polygon) is True

    def test_hexagon(self):
        import math

        angle = 0
        polygon = []
        for i in range(6):
            x = 10 * math.cos(angle)
            y = 10 * math.sin(angle)
            polygon.append((x, y))
            angle += math.pi / 3
        assert is_polygon_convex(polygon) is True


class TestCleanPolygon:
    def test_valid_triangle(self):
        polygon = P((0, 0), (10, 0), (5, 5))
        cleaned = clean_polygon(polygon)
        assert cleaned is not None
        assert len(cleaned) == 3

    def test_empty(self):
        assert clean_polygon(cast(Polygon, [])) is None

    def test_single_point(self):
        assert clean_polygon(P((0, 0))) is None

    def test_two_points(self):
        assert clean_polygon(P((0, 0), (1, 1))) is None

    def test_area_preserved_within_tolerance(self):
        """clean_polygon must preserve the polygon area within tolerance."""
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        cleaned = clean_polygon(polygon, tolerance=0.01)
        assert cleaned is not None
        original_area = abs(get_polygon_signed_area(polygon))
        cleaned_area = abs(get_polygon_signed_area(cleaned))
        assert abs(original_area - cleaned_area) < 0.01

    def test_colinear_points_removed_shape_preserved(self):
        """Removing redundant colinear bumps preserves the overall shape."""
        polygon = P(
            (0, 0),
            (5, 5),
            (10, 0),
            (15, 8),
            (20, 0),
            (20, 10),
            (0, 10),
        )
        cleaned = clean_polygon(polygon, tolerance=0.5)
        assert cleaned is not None
        assert len(cleaned) >= 4
        # Area must not change significantly
        original_area = abs(get_polygon_signed_area(polygon))
        cleaned_area = abs(get_polygon_signed_area(cleaned))
        assert abs(original_area - cleaned_area) < 0.5


class TestPolygonOffset:
    def test_expand(self):
        polygon = P((0, 0), (10, 0), (5, 10))
        offset_polys = offset_polygon(polygon, 1.0)
        assert len(offset_polys) >= 1
        expanded_area = abs(get_polygon_area(offset_polys[0]))
        original_area = abs(get_polygon_area(polygon))
        assert expanded_area > original_area

    def test_shrink(self):
        polygon = P((0, 0), (10, 0), (5, 10))
        offset_polys = offset_polygon(polygon, -0.5)
        assert len(offset_polys) >= 1
        shrunk_area = abs(get_polygon_area(offset_polys[0]))
        original_area = abs(get_polygon_area(polygon))
        assert shrunk_area < original_area

    def test_zero_offset(self):
        polygon = P((0, 0), (10, 0), (5, 10))
        offset_polys = offset_polygon(polygon, 0)
        assert len(offset_polys) == 1
        assert offset_polys[0] == polygon

    def test_empty(self):
        assert offset_polygon(cast(Polygon, []), 1.0) == []

    def test_degenerate_less_than_3_points(self):
        assert offset_polygon(P((0, 0), (1, 0)), 0.1) == []

    def test_join_style_default_miter(self):
        """offset_polygon defaults to miter join style (backward compat)."""
        polygon = P((0, 0), (10, 0), (5, 10))
        miter = offset_polygon(polygon, 1.0)
        explicit = offset_polygon(polygon, 1.0, join_style="miter")
        assert miter == explicit

    def test_join_style_round(self):
        """Round join style produces different geometry from miter."""
        polygon = P((0, 0), (10, 0), (5, 10))
        miter = offset_polygon(polygon, 1.0, join_style="miter")
        round_ = offset_polygon(polygon, 1.0, join_style="round")
        # Round joins should produce more points than miter joins
        assert len(round_[0]) > len(miter[0])

    def test_join_style_square(self):
        """Square join style should succeed without error."""
        polygon = P((0, 0), (10, 0), (5, 10))
        result = offset_polygon(polygon, 1.0, join_style="square")
        assert len(result) >= 1

    def test_join_style_invalid(self):
        """Invalid join_style should raise ValueError."""
        polygon = P((0, 0), (10, 0), (5, 10))
        with pytest.raises(ValueError, match="invalid join_style"):
            offset_polygon(polygon, 1.0, join_style="nonexistent")


class TestApplyMinimumCurvature:
    def test_basic_fillet(self):
        """Triangle with sharp corner gets filleted (more points after)."""
        poly = P((0, 0), (10, 0), (5, 10))
        result = apply_minimum_curvature(poly, 1.0)
        assert len(result) >= 1
        assert len(result[0]) > 3

    def test_positive_r_min(self):
        """r_min=0 returns the polygon unchanged (no offset)."""
        poly = P((0, 0), (10, 0), (5, 10))
        result = apply_minimum_curvature(poly, 0.0)
        assert len(result) == 1
        assert result[0] == poly

    def test_negative_r_min(self):
        """Negative r_min is clamped; same as zero."""
        poly = P((0, 0), (10, 0), (5, 10))
        result = apply_minimum_curvature(poly, -1.0)
        assert len(result) == 1
        assert result[0] == poly

    def test_degenerate(self):
        """Very large r_min can collapse the polygon."""
        poly = P((0, 0), (10, 0), (5, 10))
        result = apply_minimum_curvature(poly, 100.0)
        # May collapse to empty
        assert isinstance(result, list)


class TestPolygonBooleanOps:
    def test_union(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
        result = get_polygons_union([poly1, poly2])
        assert len(result) >= 1

    def test_union_empty(self):
        result = get_polygons_union([])
        assert result == []

    def test_intersection(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
        result = get_polygons_intersection(poly1, poly2)
        assert len(result) >= 1
        expected_area = 25.0
        actual_area = sum(abs(get_polygon_area(p)) for p in result)
        assert abs(actual_area - expected_area) < 0.1

    def test_no_intersection(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((20, 20), (30, 20), (30, 30), (20, 30))
        result = get_polygons_intersection(poly1, poly2)
        assert len(result) == 0

    def test_difference(self):
        poly1 = P((0, 0), (20, 0), (20, 20), (0, 20))
        poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
        result = get_polygons_difference(poly1, poly2)
        assert len(result) >= 1

    def test_difference_non_overlapping(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((100, 100), (110, 100), (110, 110), (100, 110))
        result = get_polygons_difference(poly1, poly2)
        assert len(result) == 1
        assert len(result[0]) == 4


class TestGetPolygonsGroupIntersection:
    def test_overlapping_rects(self):
        subject = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        clip = [P((5, 5), (15, 5), (15, 15), (5, 15))]
        result = get_polygons_group_intersection(subject, clip)
        assert len(result) >= 1
        area = sum(abs(get_polygon_area(p)) for p in result)
        assert abs(area - 25.0) < 0.1

    def test_no_overlap(self):
        subject = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        clip = [P((20, 20), (30, 20), (30, 30), (20, 30))]
        result = get_polygons_group_intersection(subject, clip)
        assert len(result) == 0

    def test_subject_inside_clip(self):
        subject = [P((2, 2), (8, 2), (8, 8), (2, 8))]
        clip = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        result = get_polygons_group_intersection(subject, clip)
        assert len(result) >= 1
        area = sum(abs(get_polygon_area(p)) for p in result)
        assert abs(area - 36.0) < 0.1

    def test_empty_subject(self):
        clip = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        result = get_polygons_group_intersection([], clip)
        assert len(result) == 0

    def test_empty_clip(self):
        subject = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        result = get_polygons_group_intersection(subject, [])
        assert len(result) == 0

    def test_multiple_subject_polygons(self):
        subject = [
            P((0, 0), (5, 0), (5, 5), (0, 5)),
            P((5, 5), (10, 5), (10, 10), (5, 10)),
        ]
        clip = [P((2, 2), (8, 2), (8, 8), (2, 8))]
        result = get_polygons_group_intersection(subject, clip)
        assert len(result) >= 1
        area = sum(abs(get_polygon_area(p)) for p in result)
        assert abs(area - 18.0) < 0.5

    def test_touching_edges(self):
        subject = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        clip = [P((10, 0), (20, 0), (20, 10), (10, 10))]
        result = get_polygons_group_intersection(subject, clip)
        assert len(result) == 0


class TestGetPolygonsGroupDifference:
    def test_subtract_inner_rect(self):
        subject = [P((0, 0), (20, 0), (20, 20), (0, 20))]
        clip = [P((5, 5), (15, 5), (15, 15), (5, 15))]
        result = get_polygons_group_difference(subject, clip)
        assert len(result) >= 1
        signed_area = sum(get_polygon_signed_area(p) for p in result)
        assert abs(signed_area - 300.0) < 1.0  # 400 - 100

    def test_no_overlap(self):
        subject = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        clip = [P((20, 20), (30, 20), (30, 30), (20, 30))]
        result = get_polygons_group_difference(subject, clip)
        assert len(result) >= 1
        area = sum(abs(get_polygon_area(p)) for p in result)
        assert abs(area - 100.0) < 0.1

    def test_clip_fully_covers_subject(self):
        subject = [P((2, 2), (8, 2), (8, 8), (2, 8))]
        clip = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        result = get_polygons_group_difference(subject, clip)
        assert len(result) == 0

    def test_empty_subject(self):
        clip = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        result = get_polygons_group_difference([], clip)
        assert len(result) == 0

    def test_empty_clip_returns_subject(self):
        subject = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        result = get_polygons_group_difference(subject, [])
        assert len(result) >= 1
        area = sum(abs(get_polygon_area(p)) for p in result)
        assert abs(area - 100.0) < 0.1

    def test_multiple_clip_polygons(self):
        subject = [P((0, 0), (20, 0), (20, 20), (0, 20))]
        clip = [
            P((2, 2), (8, 2), (8, 8), (2, 8)),
            P((12, 12), (18, 12), (18, 18), (12, 18)),
        ]
        result = get_polygons_group_difference(subject, clip)
        assert len(result) >= 1
        signed_area = sum(get_polygon_signed_area(p) for p in result)
        expected = 400.0 - 36.0 - 36.0  # 328
        assert abs(signed_area - expected) < 1.0

    def test_partial_overlap(self):
        subject = [P((0, 0), (10, 0), (10, 10), (0, 10))]
        clip = [P((5, 5), (15, 5), (15, 15), (5, 15))]
        result = get_polygons_group_difference(subject, clip)
        assert len(result) >= 1
        signed_area = sum(get_polygon_signed_area(p) for p in result)
        expected = 100.0 - 25.0  # full square minus intersection
        assert abs(signed_area - expected) < 1.0


class TestPointInPolygon:
    def test_inside(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        assert is_point_inside_polygon((5, 5), polygon) is True

    def test_outside(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        assert is_point_inside_polygon((15, 15), polygon) is False

    def test_on_edge(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        assert is_point_inside_polygon((5, 0), polygon) is True

    def test_empty_polygon(self):
        assert is_point_inside_polygon((5, 5), cast(Polygon, [])) is False

    def test_too_few_points(self):
        polygon = P((0, 0), (10, 0))
        assert is_point_inside_polygon((5, 5), polygon) is False

    def test_large_polygon(self):
        polygon = P((0, 0), (100, 0), (100, 100), (0, 100))
        assert is_point_inside_polygon((50, 50), polygon) is True


class TestPointInPolygonNumpy:
    def test_inside(self):
        polygon = PN((0, 0), (10, 0), (10, 10), (0, 10))
        assert point_in_polygon_numpy((5, 5), polygon) is True

    def test_outside(self):
        polygon = PN((0, 0), (10, 0), (10, 10), (0, 10))
        assert point_in_polygon_numpy((15, 15), polygon) is False

    def test_on_edge(self):
        polygon = PN((0, 0), (10, 0), (10, 10), (0, 10))
        assert point_in_polygon_numpy((5, 0), polygon) is True

    def test_empty_polygon(self):
        assert (
            point_in_polygon_numpy((5, 5), np.array([]).reshape(0, 2)) is False
        )

    def test_too_few_points(self):
        polygon = PN((0, 0), (10, 0))
        assert point_in_polygon_numpy((5, 5), polygon) is False

    def test_large_polygon(self):
        polygon = PN((0, 0), (100, 0), (100, 100), (0, 100))
        assert point_in_polygon_numpy((50, 50), polygon) is True


class TestPolygonsIntersect:
    def test_overlapping(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
        assert polygons_intersect(poly1, poly2) is True

    def test_non_overlapping(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((20, 20), (30, 20), (30, 30), (20, 30))
        assert polygons_intersect(poly1, poly2) is False

    def test_touching(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((10, 0), (20, 0), (20, 10), (10, 10))
        result = polygons_intersect(poly1, poly2)
        assert result is False

    def test_empty(self):
        assert (
            polygons_intersect(cast(Polygon, []), P((0, 0), (1, 0), (1, 1)))
            is False
        )

    def test_min_area_below_threshold(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P(
            (9.999, 9.999), (10.001, 9.999), (10.001, 10.001), (9.999, 10.001)
        )
        assert polygons_intersect(poly1, poly2, min_area=1e10) is False

    def test_min_area_above_threshold(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
        assert polygons_intersect(poly1, poly2, min_area=100) is True

    def test_min_area_zero(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
        assert polygons_intersect(poly1, poly2, min_area=0) is True

    def test_min_area_negative(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
        assert polygons_intersect(poly1, poly2, min_area=-10) is True

    def test_min_area_touching_polygons(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((10, 0), (20, 0), (20, 10), (10, 10))
        assert polygons_intersect(poly1, poly2, min_area=10) is False

    def test_min_area_small_intersection_filtered(self):
        poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = P((9.9, 9.9), (10.1, 9.9), (10.1, 10.1), (9.9, 10.1))
        assert polygons_intersect(poly1, poly2, min_area=1e15) is False

    def test_insufficient_vertices(self):
        assert (
            polygons_intersect(P((0, 0), (1, 0)), P((0, 0), (1, 0), (1, 1)))
            is False
        )
        assert (
            polygons_intersect(P((0, 0), (1, 0), (1, 1)), P((0, 0))) is False
        )


class TestPolygonsIntersectNumpy:
    def test_overlapping(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((5, 5), (15, 5), (15, 15), (5, 15))
        assert polygons_intersect_numpy(poly1, poly2) is True

    def test_non_overlapping(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((20, 20), (30, 20), (30, 30), (20, 30))
        assert polygons_intersect_numpy(poly1, poly2) is False

    def test_touching(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((10, 0), (20, 0), (20, 10), (10, 10))
        result = polygons_intersect_numpy(poly1, poly2)
        assert result is False

    def test_empty(self):
        assert (
            polygons_intersect_numpy(
                np.array([]).reshape(0, 2), PN((0, 0), (1, 0), (1, 1))
            )
            is False
        )

    def test_min_area_below_threshold(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN(
            (9.999, 9.999), (10.001, 9.999), (10.001, 10.001), (9.999, 10.001)
        )
        assert polygons_intersect_numpy(poly1, poly2, min_area=1e10) is False

    def test_min_area_above_threshold(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((5, 5), (15, 5), (15, 15), (5, 15))
        assert polygons_intersect_numpy(poly1, poly2, min_area=100) is True

    def test_min_area_zero(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((5, 5), (15, 5), (15, 15), (5, 15))
        assert polygons_intersect_numpy(poly1, poly2, min_area=0) is True

    def test_min_area_negative(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((5, 5), (15, 5), (15, 15), (5, 15))
        assert polygons_intersect_numpy(poly1, poly2, min_area=-10) is True

    def test_min_area_touching_polygons(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((10, 0), (20, 0), (20, 10), (10, 10))
        assert polygons_intersect_numpy(poly1, poly2, min_area=10) is False

    def test_min_area_small_intersection_filtered(self):
        poly1 = PN((0, 0), (10, 0), (10, 10), (0, 10))
        poly2 = PN((9.9, 9.9), (10.1, 9.9), (10.1, 10.1), (9.9, 10.1))
        assert polygons_intersect_numpy(poly1, poly2, min_area=1e15) is False

    def test_insufficient_vertices(self):
        assert (
            polygons_intersect_numpy(
                PN((0, 0), (1, 0)), PN((0, 0), (1, 0), (1, 1))
            )
            is False
        )
        assert (
            polygons_intersect_numpy(PN((0, 0), (1, 0), (1, 1)), PN((0, 0)))
            is False
        )


class TestAlmostEqual:
    def test_equal(self):
        assert is_almost_equal(1.0, 1.0) is True

    def test_close(self):
        assert is_almost_equal(1.0, 1.0 + 1e-10) is True

    def test_not_close(self):
        assert is_almost_equal(1.0, 1.1) is False

    def test_custom_tolerance(self):
        assert is_almost_equal(1.0, 1.01, tolerance=0.1) is True
        assert is_almost_equal(1.0, 1.01, tolerance=0.001) is False


class TestTranslateBounds:
    def test_positive_offset(self):
        bounds = (0, 0, 10, 10)
        result = translate_bounds(bounds, 5, 3)
        assert result == (5, 3, 15, 13)

    def test_negative_offset(self):
        bounds = (10, 10, 20, 20)
        result = translate_bounds(bounds, -5, -3)
        assert result == (5, 7, 15, 17)

    def test_zero_offset(self):
        bounds = (1, 2, 3, 4)
        result = translate_bounds(bounds, 0, 0)
        assert result == bounds

    def test_mixed_offset(self):
        bounds = (0, 0, 10, 10)
        result = translate_bounds(bounds, -2, 5)
        assert result == (-2, 5, 8, 15)


class TestNormalizePolygons:
    def test_basic_normalization(self):
        poly1 = P((10, 10), (20, 10), (15, 20))
        poly2 = P((30, 30), (40, 30), (35, 40))
        normalized, min_x, min_y = normalize_polygons([poly1, poly2])
        assert min_x == 10
        assert min_y == 10
        assert normalized[0][0] == (0.0, 0.0)

    def test_already_at_origin(self):
        polygon = P((0, 0), (10, 0), (5, 10))
        normalized, min_x, min_y = normalize_polygons([polygon])
        assert min_x == 0
        assert min_y == 0
        assert normalized[0] == polygon

    def test_empty_list(self):
        normalized, min_x, min_y = normalize_polygons(cast(List[Polygon], []))
        assert normalized == []
        assert min_x == 0.0
        assert min_y == 0.0

    def test_list_with_empty_polygons(self):
        normalized, min_x, min_y = normalize_polygons(
            [cast(Polygon, []), cast(Polygon, [])]
        )
        assert normalized == [[], []]
        assert min_x == 0.0
        assert min_y == 0.0

    def test_negative_coordinates(self):
        polygon = P((-5, -5), (5, -5), (0, 5))
        normalized, min_x, min_y = normalize_polygons([polygon])
        assert min_x == -5
        assert min_y == -5
        assert normalized[0][0] == (0.0, 0.0)

    def test_multiple_polygons_shared_origin(self):
        poly1 = P((10, 20), (20, 20), (15, 30))
        poly2 = P((5, 10), (15, 10), (10, 20))
        normalized, min_x, min_y = normalize_polygons([poly1, poly2])
        assert min_x == 5
        assert min_y == 10
        all_x = [p[0] for poly in normalized for p in poly]
        all_y = [p[1] for poly in normalized for p in poly]
        assert min(all_x) == 0.0
        assert min(all_y) == 0.0


class TestNormalizePolygonsNumpy:
    def test_basic_normalization(self):
        poly1 = PN((10, 10), (20, 10), (15, 20))
        poly2 = PN((30, 30), (40, 30), (35, 40))
        normalized, min_x, min_y = normalize_polygons_numpy([poly1, poly2])
        assert min_x == 10
        assert min_y == 10
        assert normalized[0][0, 0] == 0.0
        assert normalized[0][0, 1] == 0.0

    def test_already_at_origin(self):
        polygon = PN((0, 0), (10, 0), (5, 10))
        normalized, min_x, min_y = normalize_polygons_numpy([polygon])
        assert min_x == 0
        assert min_y == 0
        np.testing.assert_array_almost_equal(normalized[0], polygon)

    def test_empty_list(self):
        normalized, min_x, min_y = normalize_polygons_numpy([])
        assert normalized == []
        assert min_x == 0.0
        assert min_y == 0.0

    def test_list_with_empty_polygons(self):
        normalized, min_x, min_y = normalize_polygons_numpy(
            [np.array([]).reshape(0, 2), np.array([]).reshape(0, 2)]
        )
        assert len(normalized) == 2
        assert min_x == 0.0
        assert min_y == 0.0

    def test_negative_coordinates(self):
        polygon = PN((-5, -5), (5, -5), (0, 5))
        normalized, min_x, min_y = normalize_polygons_numpy([polygon])
        assert min_x == -5
        assert min_y == -5
        assert normalized[0][0, 0] == 0.0
        assert normalized[0][0, 1] == 0.0

    def test_multiple_polygons_shared_origin(self):
        poly1 = PN((10, 20), (20, 20), (15, 30))
        poly2 = PN((5, 10), (15, 10), (10, 20))
        normalized, min_x, min_y = normalize_polygons_numpy([poly1, poly2])
        assert min_x == 5
        assert min_y == 10
        all_x = [p[:, 0] for p in normalized]
        all_y = [p[:, 1] for p in normalized]
        assert min(np.min(x) for x in all_x) == 0.0
        assert min(np.min(y) for y in all_y) == 0.0


class TestPolygonPerimeter:
    def test_triangle(self):
        polygon = P((0, 0), (10, 0), (5, 5))
        perimeter = get_polygon_perimeter(polygon)
        expected = 10 + 5 * 2**0.5 * 2
        assert abs(perimeter - expected) < 0.001

    def test_square(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        perimeter = get_polygon_perimeter(polygon)
        assert abs(perimeter - 40.0) < 0.001

    def test_empty(self):
        assert get_polygon_perimeter(cast(Polygon, [])) == 0.0

    def test_single_point(self):
        assert get_polygon_perimeter(P((0, 0))) == 0.0

    def test_two_points(self):
        polygon = P((0, 0), (10, 0))
        perimeter = get_polygon_perimeter(polygon)
        assert abs(perimeter - 20.0) < 0.001


class TestPolygonPerimeterNumpy:
    def test_triangle(self):
        polygon = PN((0, 0), (10, 0), (5, 5))
        perimeter = polygon_perimeter_numpy(polygon)
        expected = 10 + 5 * 2**0.5 * 2
        assert abs(perimeter - expected) < 0.001

    def test_square(self):
        polygon = PN((0, 0), (10, 0), (10, 10), (0, 10))
        perimeter = polygon_perimeter_numpy(polygon)
        assert abs(perimeter - 40.0) < 0.001

    def test_empty(self):
        assert polygon_perimeter_numpy(np.array([]).reshape(0, 2)) == 0.0

    def test_single_point(self):
        assert polygon_perimeter_numpy(PN((0, 0))) == 0.0

    def test_two_points(self):
        polygon = PN((0, 0), (10, 0))
        perimeter = polygon_perimeter_numpy(polygon)
        assert abs(perimeter - 20.0) < 0.001


class TestPointLineDistance:
    def test_point_on_line(self):
        distance = point_line_distance((5, 5), (0, 5), (10, 5))
        assert abs(distance - 0.0) < 0.001

    def test_point_off_line_perpendicular(self):
        distance = point_line_distance((5, 10), (0, 5), (10, 5))
        assert abs(distance - 5.0) < 0.001

    def test_point_at_line_start(self):
        distance = point_line_distance((0, 5), (0, 5), (10, 5))
        assert abs(distance - 0.0) < 0.001

    def test_point_at_line_end(self):
        distance = point_line_distance((10, 5), (0, 5), (10, 5))
        assert abs(distance - 0.0) < 0.001

    def test_point_beyond_segment(self):
        distance = point_line_distance((-5, 5), (0, 5), (10, 5))
        assert abs(distance - 5.0) < 0.001

    def test_point_beyond_segment_end(self):
        distance = point_line_distance((15, 5), (0, 5), (10, 5))
        assert abs(distance - 5.0) < 0.001

    def test_zero_length_segment(self):
        distance = point_line_distance((5, 5), (0, 0), (0, 0))
        assert abs(distance - 5 * 2**0.5) < 0.001


class TestExtractPolygonEdges:
    def test_triangle(self):
        polygon = P((0, 0), (10, 0), (5, 5))
        edges = get_polygon_edges(polygon)
        assert len(edges) == 3
        assert (0, 0) in [e[0] for e in edges]
        assert (10, 0) in [e[0] for e in edges]
        assert (5, 5) in [e[0] for e in edges]

    def test_square(self):
        polygon = P((0, 0), (10, 0), (10, 10), (0, 10))
        edges = get_polygon_edges(polygon)
        assert len(edges) == 4
        assert (0, 0) in [e[0] for e in edges]
        assert (10, 0) in [e[0] for e in edges]
        assert (10, 10) in [e[0] for e in edges]
        assert (0, 10) in [e[0] for e in edges]

    def test_empty(self):
        edges = get_polygon_edges(cast(Polygon, []))
        assert edges == []

    def test_single_point(self):
        edges = get_polygon_edges(P((0, 0)))
        assert edges == []

    def test_two_points(self):
        polygon = P((0, 0), (10, 0))
        edges = get_polygon_edges(polygon)
        assert len(edges) == 2
        assert edges[0] == ((0, 0), (10, 0))
        assert edges[1] == ((10, 0), (0, 0))


class TestFlipPolygonNumpy:
    def test_flip_horizontal(self):
        polygon = PN((1, 2), (3, 4), (5, 6))
        flipped = flip_polygon_numpy(polygon, flip_h=True, flip_v=False)
        assert flipped[0, 0] == -1
        assert flipped[0, 1] == 2
        assert flipped[1, 0] == -3
        assert flipped[1, 1] == 4
        assert flipped[2, 0] == -5
        assert flipped[2, 1] == 6

    def test_flip_vertical(self):
        polygon = PN((1, 2), (3, 4), (5, 6))
        flipped = flip_polygon_numpy(polygon, flip_h=False, flip_v=True)
        assert flipped[0, 0] == 1
        assert flipped[0, 1] == -2
        assert flipped[1, 0] == 3
        assert flipped[1, 1] == -4
        assert flipped[2, 0] == 5
        assert flipped[2, 1] == -6

    def test_flip_both(self):
        polygon = PN((1, 2), (3, 4), (5, 6))
        flipped = flip_polygon_numpy(polygon, flip_h=True, flip_v=True)
        assert flipped[0, 0] == -1
        assert flipped[0, 1] == -2
        assert flipped[1, 0] == -3
        assert flipped[1, 1] == -4
        assert flipped[2, 0] == -5
        assert flipped[2, 1] == -6

    def test_no_flip(self):
        polygon = PN((1, 2), (3, 4))
        flipped = flip_polygon_numpy(polygon, flip_h=False, flip_v=False)
        np.testing.assert_array_almost_equal(flipped, polygon)

    def test_returns_copy(self):
        polygon = PN((1, 2), (3, 4))
        flipped = flip_polygon_numpy(polygon, flip_h=False, flip_v=False)
        assert flipped is not polygon


class TestFlipPolygonsNumpy:
    def test_flip_multiple_horizontal(self):
        poly1 = PN((1, 2), (3, 4))
        poly2 = PN((5, 6), (7, 8))
        flipped = flip_polygons_numpy(
            [poly1, poly2], flip_h=True, flip_v=False
        )
        assert len(flipped) == 2
        assert flipped[0][0, 0] == -1
        assert flipped[0][0, 1] == 2
        assert flipped[1][0, 0] == -5
        assert flipped[1][0, 1] == 6

    def test_flip_multiple_vertical(self):
        poly1 = PN((1, 2), (3, 4))
        poly2 = PN((5, 6), (7, 8))
        flipped = flip_polygons_numpy(
            [poly1, poly2], flip_h=False, flip_v=True
        )
        assert len(flipped) == 2
        assert flipped[0][0, 0] == 1
        assert flipped[0][0, 1] == -2
        assert flipped[1][0, 0] == 5
        assert flipped[1][0, 1] == -6

    def test_flip_multiple_both(self):
        poly1 = PN((1, 2), (3, 4))
        poly2 = PN((5, 6), (7, 8))
        flipped = flip_polygons_numpy([poly1, poly2], flip_h=True, flip_v=True)
        assert len(flipped) == 2
        assert flipped[0][0, 0] == -1
        assert flipped[0][0, 1] == -2
        assert flipped[1][0, 0] == -5
        assert flipped[1][0, 1] == -6

    def test_no_flip_returns_same_list(self):
        poly1 = PN((1, 2), (3, 4))
        poly2 = PN((5, 6), (7, 8))
        polygons = [poly1, poly2]
        flipped = flip_polygons_numpy(polygons, flip_h=False, flip_v=False)
        assert flipped is polygons

    def test_empty_list(self):
        flipped = flip_polygons_numpy([], flip_h=True, flip_v=True)
        assert flipped == []


def test_resample_polyline_open_path():
    points = [(0.0, 0.0, 1.0), (10.0, 0.0, 1.0)]
    resampled = resample_polyline(points, 2.0, is_closed=False)
    assert len(resampled) == 6
    assert resampled[0] == (0.0, 0.0, 1.0)
    assert resampled[-1] == (10.0, 0.0, 1.0)
    assert resampled[1] == pytest.approx((2.0, 0.0, 1.0))


def test_resample_polyline_closed_path():
    points = [
        (0.0, 0.0, 2.0),
        (10.0, 0.0, 2.0),
        (10.0, 10.0, 2.0),
        (0.0, 10.0, 2.0),
    ]
    resampled = resample_polyline(points, 5.0, is_closed=True)
    assert len(resampled) == 8
    assert resampled[0] == (0.0, 0.0, 2.0)
    assert resampled[-1] != resampled[0]
    assert (5.0, 0.0, 2.0) in resampled


# --- get_polygon_closest_point ---


def test_closest_point_on_rect():
    """Closest point to centre of a rectangle is on an edge midpoint."""
    poly = [(0, 0), (100, 0), (100, 80), (0, 80)]
    res = get_polygon_closest_point(poly, 50.0, 40.0)
    assert res is not None
    _t, (cx, cy), d2 = res
    assert abs(cy) < 1e-9 or abs(cy - 80.0) < 1e-9
    assert abs(cx - 50.0) < 1e-9


def test_closest_point_at_vertex():
    """Closest point exactly at a vertex returns distance 0."""
    poly = [(0, 0), (100, 0), (100, 80), (0, 80)]
    res = get_polygon_closest_point(poly, 0.0, 0.0)
    assert res is not None
    _t, (_cx, _cy), d2 = res
    assert d2 < 1e-12


def test_closest_point_outside():
    """Point outside still returns the closest boundary point."""
    poly = [(0, 0), (100, 0), (100, 80), (0, 80)]
    res = get_polygon_closest_point(poly, 200.0, 40.0)
    assert res is not None
    _t, (cx, cy), d2 = res
    assert abs(cx - 100.0) < 1e-9
    assert abs(cy - 40.0) < 1e-9


def test_closest_point_degenerate():
    """Degenerate polygon returns None."""
    assert get_polygon_closest_point([], 0.0, 0.0) is None
    assert get_polygon_closest_point([(0, 0)], 0.0, 0.0) is None

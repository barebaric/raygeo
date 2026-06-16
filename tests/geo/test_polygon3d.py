"""
Tests for 3D polygon boolean and offset operations.

Verifies Z preservation through all boolean and offset operations.
"""

from typing import List, Tuple

from raygeo.geo.shape.polygon3d import (
    get_polygons_difference_3d,
    get_polygons_group_difference_3d,
    get_polygons_group_intersection_3d,
    get_polygons_intersection_3d,
    get_polygons_union_3d,
    offset_polygon_3d,
)

Polygon3D = List[Tuple[float, float, float]]


def P3(*points: Tuple[float, float, float]) -> Polygon3D:
    """Helper to create a 3D polygon from point tuples."""
    return [(float(x), float(y), float(z)) for x, y, z in points]


def poly_area_xy(poly: Polygon3D) -> float:
    """Shoelace area of the XY projection (ignoring Z)."""
    n = len(poly)
    if n < 3:
        return 0.0
    s = 0.0
    for i in range(n):
        j = (i + 1) % n
        s += poly[i][0] * poly[j][1]
        s -= poly[j][0] * poly[i][1]
    return abs(s) / 2.0


# ── offset_polygon_3d ──────────────────────────────────────────────────


class TestOffsetPolygon3D:
    def test_expand_preserves_z(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        result = offset_polygon_3d(poly, 1.0)
        assert len(result) >= 1
        for p in result[0]:
            assert p[2] == 5.0

    def test_shrink_preserves_z(self):
        poly = P3((0, 0, 3), (10, 0, 3), (5, 10, 3))
        result = offset_polygon_3d(poly, -0.5)
        assert len(result) >= 1
        for p in result[0]:
            assert p[2] == 3.0

    def test_area_increases_with_positive_offset(self):
        poly = P3((0, 0, 0), (10, 0, 0), (5, 10, 0))
        expanded = offset_polygon_3d(poly, 1.0)
        assert len(expanded) >= 1
        assert poly_area_xy(expanded[0]) > poly_area_xy(poly)

    def test_area_decreases_with_negative_offset(self):
        poly = P3((0, 0, 0), (10, 0, 0), (5, 10, 0))
        shrunk = offset_polygon_3d(poly, -0.5)
        assert len(shrunk) >= 1
        assert poly_area_xy(shrunk[0]) < poly_area_xy(poly)

    def test_zero_offset(self):
        poly = P3((0, 0, 7), (10, 0, 7), (10, 10, 7), (0, 10, 7))
        result = offset_polygon_3d(poly, 0.0)
        assert len(result) == 1
        for p in result[0]:
            assert p[2] == 7.0

    def test_different_z_values(self):
        for z in [-5.0, 0.0, 12.5]:
            poly = P3((0, 0, z), (10, 0, z), (10, 10, z), (0, 10, z))
            result = offset_polygon_3d(poly, 1.0)
            assert len(result) >= 1
            for p in result[0]:
                assert p[2] == z

    def test_empty(self):
        assert offset_polygon_3d([], 1.0) == []

    def test_degenerate_less_than_3_points(self):
        assert offset_polygon_3d(P3((0, 0, 0), (1, 0, 0)), 0.1) == []


# ── get_polygons_union_3d ──────────────────────────────────────────────


class TestUnion3D:
    def test_union_preserves_first_z(self):
        poly1 = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        poly2 = P3((5, 5, 5), (15, 5, 5), (15, 15, 5), (5, 15, 5))
        result = get_polygons_union_3d([poly1, poly2])
        assert len(result) >= 1
        for p in result[0]:
            assert p[2] == 5.0

    def test_union_empty(self):
        assert get_polygons_union_3d([]) == []

    def test_union_non_overlapping(self):
        poly1 = P3((0, 0, 2), (5, 0, 2), (5, 5, 2), (0, 5, 2))
        poly2 = P3((10, 10, 2), (15, 10, 2), (15, 15, 2), (10, 15, 2))
        result = get_polygons_union_3d([poly1, poly2])
        assert len(result) >= 2

    def test_union_z_from_first_when_mixed(self):
        poly1 = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        poly2 = P3((5, 5, 99), (15, 5, 99), (15, 15, 99), (5, 15, 99))
        result = get_polygons_union_3d([poly1, poly2])
        assert len(result) >= 1
        for p in result[0]:
            assert p[2] == 5.0  # Z from poly1 (first in list)


# ── get_polygons_intersection_3d ───────────────────────────────────────


class TestIntersection3D:
    def test_intersection_preserves_z(self):
        poly1 = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        poly2 = P3((5, 5, 5), (15, 5, 5), (15, 15, 5), (5, 15, 5))
        result = get_polygons_intersection_3d(poly1, poly2)
        assert len(result) >= 1
        for p in result[0]:
            assert p[2] == 5.0
        area = poly_area_xy(result[0])
        assert abs(area - 25.0) < 0.1

    def test_no_intersection(self):
        poly1 = P3((0, 0, 1), (10, 0, 1), (10, 10, 1), (0, 10, 1))
        poly2 = P3((20, 20, 1), (30, 20, 1), (30, 30, 1), (20, 30, 1))
        result = get_polygons_intersection_3d(poly1, poly2)
        assert len(result) == 0

    def test_z_from_first_poly(self):
        poly1 = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        poly2 = P3((5, 5, 99), (15, 5, 99), (15, 15, 99), (5, 15, 99))
        result = get_polygons_intersection_3d(poly1, poly2)
        assert len(result) >= 1
        for p in result[0]:
            assert p[2] == 5.0  # Z from poly1


# ── get_polygons_difference_3d ─────────────────────────────────────────


class TestDifference3D:
    def test_difference_preserves_z(self):
        poly1 = P3((0, 0, 7), (20, 0, 7), (20, 20, 7), (0, 20, 7))
        poly2 = P3((5, 5, 7), (15, 5, 7), (15, 15, 7), (5, 15, 7))
        result = get_polygons_difference_3d(poly1, poly2)
        assert len(result) >= 1
        for p in result[0]:
            assert p[2] == 7.0

    def test_difference_non_overlapping(self):
        poly1 = P3((0, 0, 3), (10, 0, 3), (10, 10, 3), (0, 10, 3))
        poly2 = P3((100, 100, 3), (110, 100, 3), (110, 110, 3), (100, 110, 3))
        result = get_polygons_difference_3d(poly1, poly2)
        assert len(result) == 1


# ── get_polygons_group_intersection_3d ─────────────────────────────────


class TestGroupIntersection3D:
    def test_overlapping_rects(self):
        subject = [P3((0, 0, 4), (10, 0, 4), (10, 10, 4), (0, 10, 4))]
        clip = [P3((5, 5, 4), (15, 5, 4), (15, 15, 4), (5, 15, 4))]
        result = get_polygons_group_intersection_3d(subject, clip)
        assert len(result) >= 1
        area = sum(poly_area_xy(p) for p in result)
        assert abs(area - 25.0) < 0.1
        for p in result[0]:
            assert p[2] == 4.0

    def test_no_overlap(self):
        subject = [P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))]
        clip = [P3((20, 20, 0), (30, 20, 0), (30, 30, 0), (20, 30, 0))]
        assert len(get_polygons_group_intersection_3d(subject, clip)) == 0

    def test_empty_subject(self):
        clip = [P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))]
        assert len(get_polygons_group_intersection_3d([], clip)) == 0

    def test_empty_clip(self):
        subject = [P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))]
        assert len(get_polygons_group_intersection_3d(subject, [])) == 0


# ── get_polygons_group_difference_3d ───────────────────────────────────


class TestGroupDifference3D:
    def test_subtract_inner_rect(self):
        subject = [P3((0, 0, 6), (20, 0, 6), (20, 20, 6), (0, 20, 6))]
        clip = [P3((5, 5, 6), (15, 5, 6), (15, 15, 6), (5, 15, 6))]
        result = get_polygons_group_difference_3d(subject, clip)
        assert len(result) >= 1
        for p in result[0]:
            assert p[2] == 6.0

    def test_empty_subject(self):
        clip = [P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))]
        assert len(get_polygons_group_difference_3d([], clip)) == 0

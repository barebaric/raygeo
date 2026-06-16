"""
Tests for 3D polygon boolean and offset operations.

Verifies Z preservation through all boolean and offset operations.
"""

from typing import List, Tuple

from raygeo.geo.shape.polygon3d import (
    flip_polygon_3d,
    flip_polygons_3d,
    get_polygon_bounds_3d,
    get_polygon_centroid_3d,
    get_polygon_convex_hull_3d,
    get_polygon_edges_3d,
    get_polygon_group_bounds_3d,
    get_polygon_perimeter_3d,
    get_polygons_difference_3d,
    get_polygons_group_difference_3d,
    get_polygons_group_intersection_3d,
    get_polygons_intersection_3d,
    get_polygons_union_3d,
    offset_polygon_3d,
    rotate_polygon_3d,
    rotate_polygons_3d,
    scale_polygon_3d,
    translate_polygon_3d,
    translate_polygons_3d,
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


# ── 3D Analytical functions ──────────────────────────────────────────


class TestPerimeter3D:
    def test_square_perimeter(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))
        assert abs(get_polygon_perimeter_3d(poly) - 40.0) < 1e-9

    def test_3d_diagonal(self):
        poly = P3((0, 0, 0), (3, 0, 0), (3, 4, 0))
        expected = 3.0 + 4.0 + 5.0  # 5 = sqrt(3^2 + 4^2)
        assert abs(get_polygon_perimeter_3d(poly) - expected) < 1e-9

    def test_3d_z_edge_length(self):
        poly = P3((0, 0, 0), (0, 0, 5), (0, 0, 10))
        expected = 5.0 + 5.0 + 10.0  # edges: 5, 5, 10 (back to start)
        assert abs(get_polygon_perimeter_3d(poly) - expected) < 1e-9

    def test_empty(self):
        assert get_polygon_perimeter_3d([]) == 0.0

    def test_single_point(self):
        assert get_polygon_perimeter_3d([(1, 2, 3)]) == 0.0


class TestBounds3D:
    def test_basic(self):
        poly = P3((0, 0, 0), (10, 0, 5), (10, 10, 5), (0, 10, 10))
        x_min, y_min, x_max, y_max, z_min, z_max = get_polygon_bounds_3d(poly)
        assert x_min == 0.0
        assert y_min == 0.0
        assert x_max == 10.0
        assert y_max == 10.0
        assert z_min == 0.0
        assert z_max == 10.0

    def test_empty(self):
        assert get_polygon_bounds_3d([]) == (0, 0, 0, 0, 0, 0)

    def test_negative_z(self):
        poly = P3((0, 0, -5), (10, 0, -5), (10, 10, 3), (0, 10, 3))
        *_, z_min, z_max = get_polygon_bounds_3d(poly)
        assert z_min == -5.0
        assert z_max == 3.0


class TestGroupBounds3D:
    def test_basic(self):
        polys = [
            P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0)),
            P3((5, 5, 5), (15, 5, 5), (15, 15, 5), (5, 15, 5)),
        ]
        x_min, y_min, x_max, y_max, z_min, z_max = get_polygon_group_bounds_3d(
            polys
        )
        assert x_min == 0.0
        assert y_min == 0.0
        assert x_max == 15.0
        assert y_max == 15.0
        assert z_min == 0.0
        assert z_max == 5.0

    def test_empty(self):
        assert get_polygon_group_bounds_3d([]) == (0, 0, 0, 0, 0, 0)


class TestCentroid3D:
    def test_square_xy_centroid(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))
        cx, cy, cz = get_polygon_centroid_3d(poly)
        assert abs(cx - 5.0) < 1e-9
        assert abs(cy - 5.0) < 1e-9
        assert abs(cz - 0.0) < 1e-9

    def test_z_average(self):
        poly = P3((0, 0, 0), (10, 0, 2), (10, 10, 4), (0, 10, 6))
        *_, cz = get_polygon_centroid_3d(poly)
        assert abs(cz - 3.0) < 1e-9  # avg of (0+2+4+6)/4

    def test_empty(self):
        cx, cy, cz = get_polygon_centroid_3d([])
        assert cx == 0.0 and cy == 0.0 and cz == 0.0


class TestEdges3D:
    def test_square_edges(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))
        edges = get_polygon_edges_3d(poly)
        assert len(edges) == 4
        assert edges[0] == ((0, 0, 0), (10, 0, 0))
        assert edges[1] == ((10, 0, 0), (10, 10, 0))
        assert edges[2] == ((10, 10, 0), (0, 10, 0))
        assert edges[3] == ((0, 10, 0), (0, 0, 0))

    def test_empty(self):
        assert get_polygon_edges_3d([]) == []

    def test_single_point(self):
        assert get_polygon_edges_3d([(1, 2, 3)]) == []


class TestConvexHull3D:
    def test_square(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        hull = get_polygon_convex_hull_3d(poly)
        assert len(hull) == 4
        for p in hull:
            assert p[2] == 5.0

    def test_z_from_first_hull_vertex(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5), (2, 2, 99))
        hull = get_polygon_convex_hull_3d(poly)
        assert len(hull) >= 3
        for p in hull:
            assert p[2] == 5.0  # Z from first vertex

    def test_less_than_3(self):
        assert get_polygon_convex_hull_3d([(0, 0, 0), (1, 0, 0)]) == [
            (0, 0, 0),
            (1, 0, 0),
        ]

    def test_empty(self):
        assert get_polygon_convex_hull_3d([]) == []


# ── 3D Transform functions ───────────────────────────────────────────


class TestTranslate3D:
    def test_translate_xy(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = translate_polygon_3d(poly, 2.0, 3.0)
        assert result == P3((2, 3, 5), (12, 3, 5), (12, 13, 5))

    def test_translate_z(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = translate_polygon_3d(poly, 0.0, 0.0, 10.0)
        for p in result:
            assert p[2] == 15.0

    def test_translate_polygons(self):
        polys = [P3((0, 0, 0), (1, 0, 0), (0, 1, 0))]
        result = translate_polygons_3d(polys, 5.0, 5.0, 5.0)
        assert len(result) == 1
        for p in result[0]:
            assert p[2] == 5.0


class TestScale3D:
    def test_uniform_scale(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0))
        result = scale_polygon_3d(poly, 2.0)
        assert result == P3((0, 0, 0), (20, 0, 0), (20, 20, 0))

    def test_nonuniform_scale_y(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))
        result = scale_polygon_3d(poly, 2.0, scale_y=3.0)
        for p in result:
            assert p[0] in (0.0, 20.0)
            assert p[1] in (0.0, 30.0)

    def test_scale_z(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = scale_polygon_3d(poly, 1.0, scale_z=2.0)
        for p in result:
            assert p[2] == 10.0


class TestFlip3D:
    def test_flip_horizontal(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = flip_polygon_3d(poly, flip_h=True)
        assert result == P3((0, 0, 5), (-10, 0, 5), (-10, 10, 5))

    def test_flip_z(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = flip_polygon_3d(poly, flip_z=True)
        for p in result:
            assert p[2] == -5.0

    def test_flip_polygons(self):
        polys = [P3((0, 0, 3), (10, 0, 3), (10, 10, 3))]
        result = flip_polygons_3d(polys, flip_h=True, flip_z=True)
        assert len(result) == 1
        for p in result[0]:
            assert p[2] == -3.0

    def test_no_flip(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        assert flip_polygon_3d(poly) == poly


class TestRotate3D:
    def test_rotate_90_degrees(self):
        poly = P3((10, 0, 5), (20, 0, 5), (20, 10, 5))
        result = rotate_polygon_3d(poly, 90.0)
        for p in result:
            assert p[2] == 5.0  # Z preserved

    def test_rotate_360_is_identity(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        result = rotate_polygon_3d(poly, 360.0)
        for a, b in zip(poly, result):
            assert abs(a[0] - b[0]) < 1e-9
            assert abs(a[1] - b[1]) < 1e-9
            assert abs(a[2] - b[2]) < 1e-9

    def test_rotate_polygons(self):
        polys = [P3((10, 0, 5), (20, 0, 5), (20, 10, 5))]
        result = rotate_polygons_3d(polys, 180.0)
        assert len(result) == 1
        for p in result[0]:
            assert p[2] == 5.0

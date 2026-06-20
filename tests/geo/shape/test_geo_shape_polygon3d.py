"""
Tests for 3D polygon boolean and offset operations.

Verifies Z preservation through all boolean and offset operations.
"""

import math
from typing import List, Tuple

from raygeo.geo.shape.point import circumcenter
from raygeo.geo.shape.polygon3d import (
    deduplicate_polyline_3d,
    fillet_polyline_3d,
    flip_polygon_3d,
    flip_polygons_3d,
    get_polygon_area_3d,
    get_polygon_bounds_3d,
    get_polygon_centroid_3d,
    get_polygon_convex_hull_3d,
    get_polygon_edges_3d,
    get_polygon_group_bounds_3d,
    get_polygon_perimeter_3d,
    get_polygon_signed_area_3d,
    get_polygons_difference_3d,
    get_polygons_group_difference_3d,
    get_polygons_group_intersection_3d,
    get_polygons_intersection_3d,
    get_polygons_union_3d,
    get_polyline_end_tangent_3d,
    offset_polygon_3d,
    offset_polyline_3d,
    rotate_polygon_3d,
    rotate_polygons_3d,
    scale_polygon_3d,
    translate_polygon_3d,
    translate_polygons_3d,
    walk_along_polygon_3d,
    walk_along_polyline_3d,
)

Polygon3D = List[Tuple[float, float, float]]


def P3(*points: Tuple[float, float, float]) -> Polygon3D:
    """Helper to create a 3D polygon from point tuples."""
    return [(float(x), float(y), float(z)) for x, y, z in points]


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
        assert get_polygon_area_3d(expanded[0]) > get_polygon_area_3d(poly)

    def test_area_decreases_with_negative_offset(self):
        poly = P3((0, 0, 0), (10, 0, 0), (5, 10, 0))
        shrunk = offset_polygon_3d(poly, -0.5)
        assert len(shrunk) >= 1
        assert get_polygon_area_3d(shrunk[0]) < get_polygon_area_3d(poly)

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
        area = get_polygon_area_3d(result[0])
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
        area = sum(get_polygon_area_3d(p) for p in result)
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


class TestSignedArea3D:
    def test_ccw_square_positive(self):
        """CCW winding gives positive signed area."""
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))
        assert get_polygon_signed_area_3d(poly) == 100.0

    def test_cw_square_negative(self):
        """CW winding gives negative signed area."""
        poly = P3((0, 0, 0), (0, 10, 0), (10, 10, 0), (10, 0, 0))
        assert get_polygon_signed_area_3d(poly) == -100.0

    def test_reversed_sign(self):
        """Reversing a polygon flips the sign."""
        ccw = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))
        cw = ccw[::-1]
        sa_ccw = get_polygon_signed_area_3d(ccw)
        sa_cw = get_polygon_signed_area_3d(cw)
        assert abs(sa_ccw - (-sa_cw)) < 1e-9

    def test_unsigned_area_matches(self):
        """Absolute signed area equals unsigned area for CCW."""
        ccw = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))
        assert (
            abs(get_polygon_signed_area_3d(ccw) - get_polygon_area_3d(ccw))
            < 1e-9
        )

    def test_triangle(self):
        """Triangle with known signed area."""
        poly = P3((0, 0, 0), (10, 0, 0), (0, 10, 0))
        assert get_polygon_signed_area_3d(poly) == 50.0

    def test_degenerate_collinear(self):
        """Collinear points produce zero area."""
        poly = P3((0, 0, 0), (5, 0, 0), (10, 0, 0))
        assert get_polygon_signed_area_3d(poly) == 0.0

    def test_z_ignored(self):
        """Z coordinate does not affect XY signed area."""
        poly_2d = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0))
        poly_3d = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        assert (
            abs(
                get_polygon_signed_area_3d(poly_2d)
                - get_polygon_signed_area_3d(poly_3d)
            )
            < 1e-9
        )

    def test_empty(self):
        assert get_polygon_signed_area_3d([]) == 0.0

    def test_less_than_3_points(self):
        assert get_polygon_signed_area_3d([(1, 2, 3)]) == 0.0
        assert get_polygon_signed_area_3d([(1, 2, 3), (4, 5, 6)]) == 0.0


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


# ── offset_polyline_3d (true 3D offset) ──────────────────────────────


def edge_vector(a, b):
    return (b[0] - a[0], b[1] - a[1], b[2] - a[2])


def dot3(u, v):
    return u[0] * v[0] + u[1] * v[1] + u[2] * v[2]


def vec_len3(v):
    return (v[0] ** 2 + v[1] ** 2 + v[2] ** 2) ** 0.5


def edge_distance_sq(a, b, p):
    """Squared distance from point p to line segment (a,b)."""
    ab = edge_vector(a, b)
    ap = edge_vector(a, p)
    ab_len_sq = dot3(ab, ab)
    if ab_len_sq == 0:
        return dot3(ap, ap)
    t = dot3(ap, ab) / ab_len_sq
    t = max(0.0, min(1.0, t))
    proj = (
        a[0] + t * ab[0],
        a[1] + t * ab[1],
        a[2] + t * ab[2],
    )
    return dot3(edge_vector(p, proj), edge_vector(p, proj))


class TestOffsetPolyline3D:
    def test_open_xy_preserves_z(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = offset_polyline_3d(poly, 1.0)
        assert len(result) == 3
        for p in result:
            assert p[2] == 5.0

    def test_open_xy_endpoints_perpendicular(self):
        poly = P3((0, 0, 3), (10, 0, 3))
        result = offset_polyline_3d(poly, 2.0)
        assert len(result) == 2
        # Both should be offset perpendicular to the edge (0,0,3)→(10,0,3)
        # perpendicular in XY is (0, -1, 0) or (0, 1, 0)
        assert result[0][0] == 0.0
        assert result[0][2] == 3.0
        assert result[1][0] == 10.0
        assert result[1][2] == 3.0
        # The Y offset should be uniform
        assert abs(result[0][1] - result[1][1]) < 1e-9

    def test_open_xy_vertex_miter(self):
        """L-shape: verify the miter vertex is at the expected position."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = offset_polyline_3d(poly, 1.0)
        # Vertex 1 (miter): edge-plane normals (0,1,0) + (-1,0,0) = (-1,1,0)
        assert abs(result[1][0] - 9.0) < 1e-9
        assert abs(result[1][1] - 1.0) < 1e-9
        assert result[1][2] == 5.0

    def test_closed_xy_area_differs(self):
        """A closed CCW square: positive offset = left = inward = smaller."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        pos = offset_polyline_3d(poly, 1.0, closed=True)
        neg = offset_polyline_3d(poly, -1.0, closed=True)
        assert len(pos) == 4
        assert len(neg) == 4
        for p in pos + neg:
            assert p[2] == 5.0
        # For CCW: left (positive) = inward (smaller area)
        assert get_polygon_area_3d(pos) < get_polygon_area_3d(poly)
        assert get_polygon_area_3d(neg) > get_polygon_area_3d(poly)

    def test_nonplanar_open(self):
        """A 3D polyline with varying Z gets a true 3D offset."""
        poly = P3((0, 0, 0), (10, 0, 2), (10, 10, 5))
        result = offset_polyline_3d(poly, 1.0)
        assert len(result) == 3
        # The middle vertex (internal, miter) should have a different Z
        assert result[1][2] != poly[1][2]

    def test_nonplanar_closed(self):
        """A non-planar closed polygon gets miters at all vertices."""
        poly = P3((0, 0, 0), (10, 0, 2), (10, 10, 5), (0, 10, 3))
        result = offset_polyline_3d(poly, 1.0, closed=True)
        assert len(result) == 4
        # All Z values should differ (since the edges aren't horizontal)
        for p in result:
            assert p[0] != 0.0 or p[1] != 0.0 or p[2] != 0.0

    def test_zero_distance(self):
        poly = P3((0, 0, 0), (10, 0, 5), (10, 10, 10))
        assert offset_polyline_3d(poly, 0.0) == poly

    def test_single_point(self):
        assert offset_polyline_3d([(1, 2, 3)], 0.5) == [(1, 2, 3)]

    def test_empty(self):
        assert offset_polyline_3d([], 1.0) == []

    def test_two_points_closed(self):
        """Two points with closed=True gives both endpoints miters."""
        poly = P3((0, 0, 0), (10, 0, 0))
        result = offset_polyline_3d(poly, 1.0, closed=True)
        assert len(result) == 2
        # For closed, both vertices get miters (though they'll be collinear)
        for p in result:
            assert p[2] == 0.0

    def test_negative_distance(self):
        """Negative distance offsets to the opposite side as positive."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        pos = offset_polyline_3d(poly, 1.0)
        neg = offset_polyline_3d(poly, -1.0)
        # Positive and negative should be on opposite sides of the polyline
        assert pos[0][1] * neg[0][1] < 0

    def test_closed_collinear_z(self):
        """Collinear points with varying Z get perpendicular offset."""
        poly = P3((0, 0, 0), (10, 0, 5), (20, 0, 10))
        result = offset_polyline_3d(poly, 1.0)
        assert len(result) == 3
        # The middle vertex should also be perpendicular perp to the line
        for p in result:
            assert p[0] != 0.0 or p[1] != 0.0 or p[2] != 0.0

    def test_large_offset(self):
        """Large offset still produces same vertex count."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = offset_polyline_3d(poly, 100.0)
        assert len(result) == 3


# ── get_polyline_end_tangent_3d ──────────────────────────────────────


class TestEndTangent3D:
    def test_horizontal_line(self):
        poly = P3((0, 0, 0), (10, 0, 0))
        dx, dy = get_polyline_end_tangent_3d(poly)
        assert abs(dx - 1.0) < 1e-9
        assert abs(dy - 0.0) < 1e-9

    def test_vertical_line(self):
        poly = P3((0, 0, 0), (0, 10, 0))
        dx, dy = get_polyline_end_tangent_3d(poly)
        assert abs(dx - 0.0) < 1e-9
        assert abs(dy - 1.0) < 1e-9

    def test_diagonal(self):
        poly = P3((0, 0, 0), (3, 4, 0))
        dx, dy = get_polyline_end_tangent_3d(poly)
        assert abs(dx - 0.6) < 1e-9
        assert abs(dy - 0.8) < 1e-9

    def test_normalised(self):
        poly = P3((0, 0, 0), (5, 12, 0))
        dx, dy = get_polyline_end_tangent_3d(poly)
        length = math.sqrt(dx * dx + dy * dy)
        assert abs(length - 1.0) < 1e-9

    def test_lookback_uses_last_segment(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0))
        dx, dy = get_polyline_end_tangent_3d(poly)
        # Last segment is (10,0) -> (10,10), direction = (0, 1)
        assert abs(dx - 0.0) < 1e-9
        assert abs(dy - 1.0) < 1e-9

    def test_single_point_returns_default(self):
        poly = P3((5, 5, 0))
        dx, dy = get_polyline_end_tangent_3d(poly)
        assert abs(dx - 1.0) < 1e-9
        assert abs(dy - 0.0) < 1e-9

    def test_empty_returns_default(self):
        dx, dy = get_polyline_end_tangent_3d([])
        assert abs(dx - 1.0) < 1e-9
        assert abs(dy - 0.0) < 1e-9

    def test_zero_length_last_edge(self):
        poly = P3((0, 0, 0), (5, 0, 0), (5, 0, 0))
        dx, dy = get_polyline_end_tangent_3d(poly)
        assert abs(dx - 1.0) < 1e-9
        assert abs(dy - 0.0) < 1e-9


# ── deduplicate_polyline_3d ────────────────────────────────────────────


class TestDeduplicatePolyline3D:
    def test_no_duplicates(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0))
        result = deduplicate_polyline_3d(poly)
        assert result == poly

    def test_exact_consecutive_duplicates(self):
        poly = P3((0, 0, 0), (0, 0, 0), (10, 0, 0), (10, 10, 0))
        result = deduplicate_polyline_3d(poly)
        assert result == P3((0, 0, 0), (10, 0, 0), (10, 10, 0))

    def test_near_duplicates_within_tolerance(self):
        tol = 1e-9
        poly = P3((0, 0, 0), (tol, 0, 0), (10, 0, 0))
        result = deduplicate_polyline_3d(poly)
        assert len(result) == 2

    def test_barely_beyond_tolerance(self):
        tol = 1.1e-6
        poly = P3((0, 0, 0), (tol, 0, 0), (10, 0, 0))
        result = deduplicate_polyline_3d(poly)
        assert len(result) == 3  # sqrt(1.21e-12) > 1e-12 threshold

    def test_three_consecutive_duplicates(self):
        poly = P3((0, 0, 0), (0, 0, 0), (0, 0, 0), (5, 5, 5))
        result = deduplicate_polyline_3d(poly)
        assert result == P3((0, 0, 0), (5, 5, 5))

    def test_z_preserved(self):
        poly = P3((0, 0, 5), (0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = deduplicate_polyline_3d(poly)
        for p in result:
            assert p[2] == 5.0

    def test_empty(self):
        assert deduplicate_polyline_3d([]) == []

    def test_single_point(self):
        assert deduplicate_polyline_3d([(1, 2, 3)]) == [(1, 2, 3)]

    def test_all_duplicates(self):
        poly = P3((1, 2, 3), (1, 2, 3), (1, 2, 3))
        result = deduplicate_polyline_3d(poly)
        assert result == [(1.0, 2.0, 3.0)]

    def test_input_not_mutated(self):
        original = P3((0, 0, 0), (0, 0, 0), (10, 0, 0))
        copy = list(original)
        deduplicate_polyline_3d(original)
        assert original == copy  # original untouched in Python


# ── fillet_polyline_3d ──────────────────────────────────────────────────


class TestFilletPolyline3D:
    def test_right_angle_90_deg(self):
        """L-shape: (0,0)→(10,0)→(10,10) with radius 2 → arc at (10,0)."""
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0))
        result = fillet_polyline_3d(poly, 2.0)
        assert len(result) > 3
        # First and last points must be preserved
        assert result[0] == (0.0, 0.0, 0.0)
        assert result[-1] == (10.0, 10.0, 0.0)
        # Tangent point on the horizontal leg (first inserted arc point)
        assert abs(result[1][0] - 8.0) < 1e-9
        assert abs(result[1][1] - 0.0) < 1e-9
        # Tangent point on the vertical leg (last inserted arc point)
        assert abs(result[-2][0] - 10.0) < 1e-9
        assert abs(result[-2][1] - 2.0) < 1e-9
        # Z must be preserved
        for p in result:
            assert p[2] == 0.0
        # Arc points should lie on a circle of radius 2 centered at (8, 2)
        cx, cy = 8.0, 2.0
        for p in result[1:-1]:
            d = ((p[0] - cx) ** 2 + (p[1] - cy) ** 2) ** 0.5
            assert abs(d - 2.0) < 1e-6

    def test_left_hand_corner(self):
        """Reversed L: (10,10)→(10,0)→(0,0) with radius 2."""
        poly = P3((10, 10, 0), (10, 0, 0), (0, 0, 0))
        result = fillet_polyline_3d(poly, 2.0)
        assert len(result) > 3
        assert result[0] == (10.0, 10.0, 0.0)
        assert result[-1] == (0.0, 0.0, 0.0)
        # Tangent on vertical leg: (10, -2) but clamped to (10,2) since
        # we go down from 10 to 0, then left.  The tangent point on the
        # outgoing edge (going left) is at (8,0).
        # tangent offset d = 2/tan45° = 2
        # t_in = curr + u_in * d = (10,0) + (0,-1)*2 = (10, -2) ... wait
        # u_in = prev - curr = (0,10), normalized = (0,1)
        # t_in = (10,0) + (0,1)*2 = (10, 2)
        assert abs(result[1][0] - 10.0) < 1e-9
        assert abs(result[1][1] - 2.0) < 1e-9
        # Tangent on horizontal outgoing: (8, 0)
        assert abs(result[-2][0] - 8.0) < 1e-9
        assert abs(result[-2][1] - 0.0) < 1e-9
        for p in result:
            assert p[2] == 0.0

    def test_very_short_segment_skips_fillet(self):
        """When tan_off exceeds segment length, corner is kept sharp."""
        poly = P3((0, 0, 0), (1, 0, 0), (1, 10, 0))
        result = fillet_polyline_3d(poly, 5.0)
        # Fillet radius is too large for the 1-unit segment → unchanged
        assert len(result) == 3
        assert result == poly

    def test_radius_too_large_skips_fillet(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0))
        result = fillet_polyline_3d(poly, 100.0)
        assert len(result) == 3
        assert result == poly

    def test_zero_radius_returns_unchanged(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0))
        result = fillet_polyline_3d(poly, 0.0)
        assert result == poly

    def test_negative_radius_returns_unchanged(self):
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0))
        result = fillet_polyline_3d(poly, -1.0)
        assert result == poly

    def test_empty_returns_empty(self):
        assert fillet_polyline_3d([], 2.0) == []

    def test_single_point(self):
        assert fillet_polyline_3d([(1, 2, 3)], 2.0) == [(1, 2, 3)]

    def test_two_points(self):
        poly = P3((0, 0, 0), (10, 0, 0))
        assert fillet_polyline_3d(poly, 2.0) == poly

    def test_z_preserved(self):
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = fillet_polyline_3d(poly, 2.0)
        for p in result:
            assert p[2] == 5.0

    def test_acute_angle(self):
        """A sharp corner still gets filleted if segments are long enough."""
        # (0,0)→(20,0)→(18,2) — a ~5.7° turn, segments long enough
        poly = P3((0, 0, 0), (20, 0, 0), (18, 2, 0))
        result = fillet_polyline_3d(poly, 1.0)
        assert len(result) > 3
        for p in result:
            assert p[2] == 0.0

    def test_multiple_corners(self):
        """Four-corner polyline, each corner gets filleted if possible."""
        poly = P3(
            (0, 0, 0),
            (10, 0, 0),
            (10, 10, 0),
            (20, 10, 0),
            (20, 20, 0),
        )
        result = fillet_polyline_3d(poly, 1.0)
        # Each of the 3 internal corners gets filleted (≥2 pts each)
        assert len(result) >= 3 + 3 * 2
        # First and last preserved
        assert result[0] == (0.0, 0.0, 0.0)
        assert result[-1] == (20.0, 20.0, 0.0)
        for p in result:
            assert p[2] == 0.0

    def test_non_planar_true_3d_fillet(self):
        """Non-planar polyline: arc lies in each corner's edge plane.

        Regression test for the bug where t_in/t_out were computed with 3D
        edge vectors but the arc was drawn in the XY plane at Z=curr.z,
        producing Z jumps at t_in and XY discontinuities at t_out.
        """
        prev, curr, next_ = (0.0, 0.0, 0.0), (8.0, 0.0, 0.0), (8.0, 6.0, 3.0)
        poly = P3(prev, curr, next_)
        result = fillet_polyline_3d(poly, 1.5)
        assert len(result) > 3
        # Endpoints preserved
        assert result[0] == prev
        assert result[-1] == next_

        t_in = result[1]
        t_out = result[-2]
        arc_pts = result[2:-2]

        # t_in must lie exactly on the incoming edge prev->curr (Z=0 here).
        assert abs(t_in[2] - 0.0) < 1e-9
        assert abs(t_in[1] - 0.0) < 1e-9
        # t_out must lie on the outgoing edge curr->next, whose direction
        # is (0, 6, 3). At distance tan_off=1.5 from curr the Z step is
        # 1.5 * 3/sqrt(45) ≈ 0.671 — definitely nonzero, proving the arc
        # is not clamped to curr.z.
        assert abs(t_out[0] - 8.0) < 1e-9
        assert abs(t_out[2] - (3.0 * 1.5 / math.sqrt(45.0))) < 1e-9

        # All arc points + t_in + t_out are co-circular: recover the center
        # as the point equidistant from t_in, t_out and the midpoint arc
        # point, then verify every arc point is at distance `radius`.
        def dist(p, q):
            return math.sqrt(
                (p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2 + (p[2] - q[2]) ** 2
            )

        assert len(arc_pts) >= 1
        mid = arc_pts[len(arc_pts) // 2]
        center = circumcenter(t_in, mid, t_out)
        assert center is not None, "arc points are collinear"
        for p in result[1:-1]:
            assert abs(dist(p, center) - 1.5) < 1e-6, (
                f"point {p} not on fillet circle (d={dist(p, center)})"
            )

    def test_arc_lies_in_edge_plane(self):
        """For a non-planar corner, every arc point must lie in the plane
        defined by prev, curr, next (the edge plane)."""
        prev, curr, next_ = (0.0, 0.0, 0.0), (8.0, 0.0, 0.0), (8.0, 6.0, 3.0)
        poly = P3(prev, curr, next_)
        result = fillet_polyline_3d(poly, 1.5)

        def cross(ax, ay, az, bx, by, bz):
            return (
                ay * bz - az * by,
                az * bx - ax * bz,
                ax * by - ay * bx,
            )

        def sub(a, b):
            return (a[0] - b[0], a[1] - b[1], a[2] - b[2])

        def dot(a, b):
            return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]

        # Plane normal from the two edge vectors.
        n = cross(*sub(curr, prev), *sub(next_, curr))
        # Every arc point P must satisfy n . (P - prev) ≈ 0.
        for p in result:
            d = dot(n, sub(p, prev))
            assert abs(d) < 1e-9, f"arc point {p} not in edge plane (d={d})"


# ── walk_along_polyline_3d ────────────────────────────────────────────


class TestWalkAlongPolyline3D:
    def test_forward_partial_segment(self):
        """Walk forward part way along the first segment."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (2, 0, 5), forward=True, distance=3.0
        )
        assert result == (5, 0, 5)

    def test_forward_to_next_vertex(self):
        """Walk forward exactly one full segment."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (0, 0, 5), forward=True, distance=10.0
        )
        assert result == (10, 0, 5)

    def test_forward_crosses_segment_boundary(self):
        """Walk forward across a vertex into the next segment."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (0, 0, 5), forward=True, distance=15.0
        )
        assert result == (10, 5, 5)

    def test_forward_clamps_at_end(self):
        """Walk forward past the last point clamps to the last point."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (0, 0, 5), forward=True, distance=1000.0
        )
        assert result == (10, 10, 5)

    def test_forward_from_last_point_stays(self):
        """Walking forward from the last point stays at the last point."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (10, 10, 5), forward=True, distance=5.0
        )
        assert result == (10, 10, 5)

    def test_backward_partial_segment(self):
        """Walk backward part way along a segment."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (2, 0, 5), forward=False, distance=1.0
        )
        assert result == (1, 0, 5)

    def test_backward_to_prev_vertex(self):
        """Walk backward exactly one full segment."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (10, 0, 5), forward=False, distance=10.0
        )
        assert result == (0, 0, 5)

    def test_backward_crosses_segment_boundary(self):
        """Walk backward across a vertex into the previous segment."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (10, 0, 5), forward=False, distance=15.0
        )
        assert result == (0, 0, 5)

    def test_backward_clamps_at_start(self):
        """Walk backward past the first point clamps to the first point."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (10, 0, 5), forward=False, distance=1000.0
        )
        assert result == (0, 0, 5)

    def test_backward_from_first_point_stays(self):
        """Walking backward from the first point stays at the first point."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        result = walk_along_polyline_3d(
            poly, (0, 0, 5), forward=False, distance=5.0
        )
        assert result == (0, 0, 5)

    def test_forward_and_backward_cancel(self):
        """Walking forward then backward the same distance returns to start."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        start = (5, 0, 5)
        mid = walk_along_polyline_3d(poly, start, forward=True, distance=10.0)
        back = walk_along_polyline_3d(poly, mid, forward=False, distance=10.0)
        for a, b in zip(back, start):
            assert abs(a - b) < 1e-12

    def test_zero_distance_returns_start(self):
        """Zero distance returns the start point unchanged."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5))
        start = (3, 0, 5)
        result = walk_along_polyline_3d(
            poly, start, forward=True, distance=0.0
        )
        assert result == start

    def test_z_preserved(self):
        """The result point has the same Z value as the polyline."""
        poly = P3((0, 0, 7), (10, 0, 7), (10, 10, 7))
        start = (0, 0, 7)
        result = walk_along_polyline_3d(
            poly, start, forward=True, distance=5.0
        )
        assert result[2] == 7.0

    def test_result_lies_on_polyline(self):
        """The result point always lies on some segment of the polyline."""
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (5, 5, 0))
        for start in [(0, 0, 0), (3, 0, 0), (10, 5, 0)]:
            for d in [0.0, 2.0, 10.0, 100.0]:
                result = walk_along_polyline_3d(
                    poly, start, forward=True, distance=d
                )
                on_seg = False
                for i in range(len(poly) - 1):
                    a = poly[i]
                    b = poly[i + 1]
                    ab = (b[0] - a[0], b[1] - a[1])
                    ap = (result[0] - a[0], result[1] - a[1])
                    ab_len_sq = ab[0] ** 2 + ab[1] ** 2
                    if ab_len_sq < 1e-12:
                        continue
                    t = (ap[0] * ab[0] + ap[1] * ab[1]) / ab_len_sq
                    if t < -1e-9 or t > 1.0 + 1e-9:
                        continue
                    closest = (a[0] + t * ab[0], a[1] + t * ab[1])
                    dsq = (result[0] - closest[0]) ** 2 + (
                        result[1] - closest[1]
                    ) ** 2
                    if dsq < 1e-18:
                        on_seg = True
                        break
                assert on_seg, f"Result {result} not on polyline for d={d}"

    def test_3d_diagonal_segment(self):
        """Walk along a 3D diagonal segment with non-zero Z."""
        poly = P3((0, 0, 0), (3, 4, 12), (10, 0, 5))
        start = (0, 0, 0)
        edge_len = 13.0  # sqrt(3^2 + 4^2 + 12^2)
        result = walk_along_polyline_3d(
            poly, start, forward=True, distance=edge_len
        )
        assert abs(result[0] - 3.0) < 1e-9
        assert abs(result[1] - 4.0) < 1e-9
        assert abs(result[2] - 12.0) < 1e-9

    def test_2_point_polyline(self):
        """A 2-vertex polyline works."""
        poly = P3((0, 0, 0), (10, 0, 0))
        start = (0, 0, 0)
        result = walk_along_polyline_3d(
            poly, start, forward=True, distance=5.0
        )
        assert result == (5, 0, 0)
        result = walk_along_polyline_3d(
            poly, start, forward=True, distance=10.0
        )
        assert result == (10, 0, 0)


# ── walk_along_polygon_3d ─────────────────────────────────────────────


class TestWalkAlongPolygon3D:
    def test_forward_partial_segment(self):
        """Walk forward part way along the first edge."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        result = walk_along_polygon_3d(
            poly, (2, 0, 5), forward=True, distance=3.0
        )
        assert result == (5, 0, 5)

    def test_forward_to_next_vertex(self):
        """Walk forward exactly one full edge."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        result = walk_along_polygon_3d(
            poly, (0, 0, 5), forward=True, distance=10.0
        )
        assert result == (10, 0, 5)

    def test_forward_crosses_segment_boundary(self):
        """Walk forward across a vertex into the next edge."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        result = walk_along_polygon_3d(
            poly, (0, 0, 5), forward=True, distance=15.0
        )
        assert result == (10, 5, 5)

    def test_forward_wraps_around(self):
        """Walk forward past the last vertex wraps to the first edge."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        # perimeter = 10+10+10+10 = 40
        # walk 45 from (0,0,5) → wraps around and ends 5 into edge 0
        result = walk_along_polygon_3d(
            poly, (0, 0, 5), forward=True, distance=45.0
        )
        assert result == (5, 0, 5)

    def test_forward_full_perimeter_returns_to_start(self):
        """Walking exactly one full perimeter returns to the start."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        perim = get_polygon_perimeter_3d(poly)
        result = walk_along_polygon_3d(
            poly, (3, 0, 5), forward=True, distance=perim
        )
        assert result[0] == 3.0
        assert result[2] == 5.0

    def test_forward_multiple_perimeters(self):
        """Walking several full perimeters lands at the same place."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        perim = get_polygon_perimeter_3d(poly)
        start = (3, 0, 5)
        d1 = walk_along_polygon_3d(
            poly, start, forward=True, distance=perim + 7.0
        )
        d2 = walk_along_polygon_3d(poly, start, forward=True, distance=7.0)
        for a, b in zip(d1, d2):
            assert abs(a - b) < 1e-12

    def test_backward_partial_segment(self):
        """Walk backward part way along an edge."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        result = walk_along_polygon_3d(
            poly, (2, 0, 5), forward=False, distance=1.0
        )
        assert result == (1, 0, 5)

    def test_backward_to_prev_vertex(self):
        """Walk backward exactly one full edge."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        result = walk_along_polygon_3d(
            poly, (10, 0, 5), forward=False, distance=10.0
        )
        assert result == (0, 0, 5)

    def test_backward_crosses_segment_boundary(self):
        """Walk backward across a vertex into the previous edge."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        # From (10,0,5): 10 units back along edge 0 → (0,0), then 5 units
        # back along edge 3 ((0,10)→(0,0) direction = toward (0,10))
        result = walk_along_polygon_3d(
            poly, (10, 0, 5), forward=False, distance=15.0
        )
        assert result == (0, 5, 5)

    def test_backward_wraps_around(self):
        """Walk backward past the first vertex wraps to the last edge."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        # 45 % 40 = 5. From (0,0,5), walk 5 backward along edge 3
        # ((0,10)→(0,0) direction = toward (0,10))
        result = walk_along_polygon_3d(
            poly, (0, 0, 5), forward=False, distance=45.0
        )
        assert result == (0, 5, 5)

    def test_backward_full_perimeter_returns_to_start(self):
        """Walking backward one full perimeter returns to the start."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        perim = get_polygon_perimeter_3d(poly)
        start = (3, 0, 5)
        result = walk_along_polygon_3d(
            poly, start, forward=False, distance=perim
        )
        for a, b in zip(result, start):
            assert abs(a - b) < 1e-12

    def test_forward_and_backward_cancel(self):
        """Walking forward then backward the same distance returns to start."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        start = (5, 0, 5)
        mid = walk_along_polygon_3d(poly, start, forward=True, distance=10.0)
        back = walk_along_polygon_3d(poly, mid, forward=False, distance=10.0)
        for a, b in zip(back, start):
            assert abs(a - b) < 1e-12

    def test_zero_distance_returns_start(self):
        """Zero distance returns the start point unchanged."""
        poly = P3((0, 0, 5), (10, 0, 5), (10, 10, 5), (0, 10, 5))
        start = (3, 0, 5)
        result = walk_along_polygon_3d(poly, start, forward=True, distance=0.0)
        assert result == start

    def test_z_preserved(self):
        """Result point has the same Z as the polygon."""
        poly = P3((0, 0, 7), (10, 0, 7), (10, 10, 7), (0, 10, 7))
        result = walk_along_polygon_3d(
            poly, (0, 0, 7), forward=True, distance=5.0
        )
        assert result[2] == 7.0

    def test_result_lies_on_polygon(self):
        """Result always lies on some edge of the polygon."""
        poly = P3((0, 0, 0), (10, 0, 0), (10, 10, 0), (5, 5, 0))
        n = len(poly)
        for start in [(0, 0, 0), (3, 0, 0), (10, 5, 0)]:
            for d in [0.0, 2.0, 10.0, 42.0, 100.0]:
                result = walk_along_polygon_3d(
                    poly, start, forward=True, distance=d
                )
                on_seg = False
                for i in range(n):
                    a = poly[i]
                    b = poly[(i + 1) % n]
                    ab = (b[0] - a[0], b[1] - a[1])
                    ap = (result[0] - a[0], result[1] - a[1])
                    ab_len_sq = ab[0] ** 2 + ab[1] ** 2
                    if ab_len_sq < 1e-12:
                        continue
                    t = (ap[0] * ab[0] + ap[1] * ab[1]) / ab_len_sq
                    if t < -1e-9 or t > 1.0 + 1e-9:
                        continue
                    closest = (a[0] + t * ab[0], a[1] + t * ab[1])
                    dsq = (result[0] - closest[0]) ** 2 + (
                        result[1] - closest[1]
                    ) ** 2
                    if dsq < 1e-18:
                        on_seg = True
                        break
                assert on_seg, f"Result {result} not on polygon for d={d}"

    def test_3d_diagonal_segment(self):
        """Walk along a 3D diagonal edge."""
        poly = P3((0, 0, 0), (3, 4, 12), (10, 0, 5), (0, 10, 5))
        start = (0, 0, 0)
        edge_len = 13.0  # sqrt(3^2 + 4^2 + 12^2)
        result = walk_along_polygon_3d(
            poly, start, forward=True, distance=edge_len
        )
        assert abs(result[0] - 3.0) < 1e-9
        assert abs(result[1] - 4.0) < 1e-9
        assert abs(result[2] - 12.0) < 1e-9

    def test_triangle(self):
        """A 3-vertex triangle works."""
        poly = P3((0, 0, 0), (10, 0, 0), (5, 10, 0))
        perim = get_polygon_perimeter_3d(poly)
        result = walk_along_polygon_3d(
            poly, (0, 0, 0), forward=True, distance=perim / 2
        )
        # perimeter = 10 + 2*sqrt(125) ≈ 32.361, half ≈ 16.180
        # Edge 0 length = 10, so remaining ≈ 6.180 into edge 1
        # Edge 1 (10,0)→(5,10), length = sqrt(125) ≈ 11.180
        # t = 6.180 / 11.180 ≈ 0.553
        # point = (10 + 0.553*(5-10), 0 + 0.553*(10-0)) ≈ (7.236, 5.528)
        assert abs(result[0] - 7.23606797749979) < 1e-9
        assert abs(result[1] - 5.52786404500042) < 1e-9

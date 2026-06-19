"""Tests for build_triangle_mesh: CDT meshing."""

import math

import pytest

from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.types import TriangleMesh


def _square():
    return [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]


def _centered_square_hole():
    return [(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)]


def _triangle():
    return [(0.0, 0.0), (10.0, 0.0), (5.0, 8.66)]


def _l_shape():
    return [
        (0, 0),
        (10, 0),
        (10, 3),
        (3, 3),
        (3, 10),
        (0, 10),
    ]


def _pentagon(radius=5.0, cx=5.0, cy=5.0):
    pts = []
    for i in range(5):
        angle = 2 * math.pi * i / 5 - math.pi / 2
        pts.append(
            (cx + radius * math.cos(angle), cy + radius * math.sin(angle))
        )
    return pts


def _point_in_polygon(px, py, poly):
    n = len(poly)
    inside = False
    j = n - 1
    for i in range(n):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if ((yi > py) != (yj > py)) and (
            px < (xj - xi) * (py - yi) / (yj - yi + 1e-30) + xi
        ):
            inside = not inside
        j = i
    return inside


class TestBuildTriangleMesh:
    def test_square_generates_mesh(self):
        mesh = build_triangle_mesh(_square())
        assert isinstance(mesh, TriangleMesh)
        assert len(mesh.vertices) >= 4
        assert len(mesh.triangles) > 0
        assert len(mesh.boundary_tags) == len(mesh.vertices)
        assert len(mesh.adjacency) == len(mesh.triangles) * 3

    def test_triangle_generates_mesh(self):
        mesh = build_triangle_mesh(_triangle())
        assert len(mesh.vertices) >= 3
        assert len(mesh.triangles) > 0

    def test_l_shape_generates_mesh(self):
        mesh = build_triangle_mesh(_l_shape())
        assert len(mesh.vertices) >= 6
        assert len(mesh.triangles) > 0

    def test_square_with_hole_generates_mesh(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole])
        assert len(mesh.vertices) >= 8
        assert len(mesh.triangles) > 0

    def test_pentagon_generates_mesh(self):
        outer = _pentagon()
        mesh = build_triangle_mesh(outer)
        assert len(mesh.vertices) >= 5
        assert len(mesh.triangles) > 0

    def test_all_triangles_have_valid_indices(self):
        mesh = build_triangle_mesh(_square())
        nv = len(mesh.vertices)
        for t in mesh.triangles:
            assert 0 <= t[0] < nv
            assert 0 <= t[1] < nv
            assert 0 <= t[2] < nv
            assert t[0] != t[1] != t[2] != t[0]

    def test_zero_tool_radius_no_offset(self):
        m1 = build_triangle_mesh(_square(), tool_radius=0.0)
        m2 = build_triangle_mesh(_square())
        assert len(m1.vertices) == len(m2.vertices)
        assert len(m1.triangles) == len(m2.triangles)

    def test_outer_vertices_have_clockwise_boundary_indices(self):
        mesh = build_triangle_mesh(_square())
        outer_indices = [
            i for i, t in enumerate(mesh.boundary_tags) if t == "outer"
        ]
        assert len(outer_indices) >= 4

    def test_outer_vertices_only_on_boundary(self):
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        outer_poly = _square()
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                x, y = mesh.vertices[i]
                on_edge = False
                for e in range(len(outer_poly)):
                    x1, y1 = outer_poly[e]
                    x2, y2 = outer_poly[(e + 1) % len(outer_poly)]
                    dx = x2 - x1
                    dy = y2 - y1
                    t = ((x - x1) * dx + (y - y1) * dy) / (
                        dx * dx + dy * dy + 1e-30
                    )
                    t = max(0.0, min(1.0, t))
                    dist = math.hypot(x - (x1 + t * dx), y - (y1 + t * dy))
                    if dist < 1e-4:
                        on_edge = True
                        break
                assert on_edge, f"outer vertex {(x, y)} not on outer boundary"

    def test_inner_vertices_on_inner_boundary(self):
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(_square(), [hole], min_angle=30.0)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "inner":
                x, y = mesh.vertices[i]
                on_edge = False
                for e in range(len(hole)):
                    x1, y1 = hole[e]
                    x2, y2 = hole[(e + 1) % len(hole)]
                    dx = x2 - x1
                    dy = y2 - y1
                    t = ((x - x1) * dx + (y - y1) * dy) / (
                        dx * dx + dy * dy + 1e-30
                    )
                    t = max(0.0, min(1.0, t))
                    dist = math.hypot(x - (x1 + t * dx), y - (y1 + t * dy))
                    if dist < 1e-4:
                        on_edge = True
                        break
                assert on_edge, f"inner vertex {(x, y)} not on hole boundary"

    def test_tool_radius_reduces_outer_region(self):
        m_small = build_triangle_mesh(_square(), [], tool_radius=1.5)
        outer_poly = _square()
        for x, y in m_small.vertices[:4]:
            assert _point_in_polygon(x, y, outer_poly)

    def test_min_angle_affects_mesh_density(self):
        coarse = build_triangle_mesh(_square(), min_angle=40.0)
        fine = build_triangle_mesh(_square(), min_angle=15.0)
        assert len(fine.vertices) >= len(coarse.vertices)
        assert len(fine.triangles) >= len(coarse.triangles)


class TestBuildTriangleMeshErrors:
    def test_too_few_outer_vertices(self):
        with pytest.raises(ValueError):
            build_triangle_mesh([(0, 0), (10, 0)])

    def test_too_few_hole_vertices(self):
        with pytest.raises(ValueError):
            build_triangle_mesh(_square(), [[(1, 1), (2, 2)]])

    def test_empty_outer(self):
        with pytest.raises(ValueError):
            build_triangle_mesh([])

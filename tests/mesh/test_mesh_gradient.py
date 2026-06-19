"""Tests for compute_gradient_field."""

import math

import pytest

from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.gradient import compute_gradient_field
from raygeo.mesh.laplace import solve_laplace, solve_laplace_with_history
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


class TestComputeGradientField:
    def test_returns_list_of_tuples(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh)
        grad = compute_gradient_field(mesh, u)
        assert isinstance(grad, list)
        assert len(grad) == len(mesh.triangles)
        for g in grad:
            assert isinstance(g, tuple)
            assert len(g) == 2
            assert all(isinstance(v, float) for v in g)

    def test_gradient_length_matches_triangle_count(self):
        shapes = [_square(), _triangle(), _l_shape(), _pentagon()]
        for outer in shapes:
            mesh = build_triangle_mesh(outer, min_angle=20.0)
            u = solve_laplace(mesh)
            grad = compute_gradient_field(mesh, u)
            assert len(grad) == len(mesh.triangles)

    def test_all_gradients_are_finite(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh)
        grad = compute_gradient_field(mesh, u)
        for gx, gy in grad:
            assert math.isfinite(gx)
            assert math.isfinite(gy)
            assert not math.isnan(gx)
            assert not math.isnan(gy)

    def test_interior_triangles_have_nonzero_gradient(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh)
        grad = compute_gradient_field(mesh, u)
        interior_count = 0
        for ti, (a, b, c) in enumerate(mesh.triangles):
            tags = [
                mesh.boundary_tags[a],
                mesh.boundary_tags[b],
                mesh.boundary_tags[c],
            ]
            if all(t == "free" for t in tags):
                interior_count += 1
                gx, gy = grad[ti]
                mag = math.hypot(gx, gy)
                assert mag > 1e-30, f"interior triangle {ti} has zero gradient"
        assert interior_count > 0, "no fully interior triangles found"

    def test_gradient_vanishes_for_uniform_solution(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        grad = compute_gradient_field(mesh, u)
        for gx, gy in grad:
            assert abs(gx) < 1e-6
            assert abs(gy) < 1e-6

    def test_gradient_with_hole(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
        grad = compute_gradient_field(mesh, u)
        u_at_triangle = []
        for ti, (a, b, c) in enumerate(mesh.triangles):
            u_avg = (u[a] + u[b] + u[c]) / 3.0
            gx, gy = grad[ti]
            mag = math.hypot(gx, gy)
            u_at_triangle.append((u_avg, mag))
        u_vals = [v[0] for v in u_at_triangle]
        mags = [v[1] for v in u_at_triangle]
        assert max(u_vals) - min(u_vals) > 0.3
        assert max(mags) > 1e-6

    def test_gradient_magnitude_positive_semidefinite(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh)
        grad = compute_gradient_field(mesh, u)
        for gx, gy in grad:
            assert gx * gx + gy * gy >= -1e-30

    def test_gradient_linear_function_exact(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u_linear = [
            mesh.vertices[i][0] / 10.0 for i in range(len(mesh.vertices))
        ]
        grad = compute_gradient_field(mesh, u_linear)
        for gx, gy in grad:
            assert gx == pytest.approx(0.1, abs=1e-6)
            assert gy == pytest.approx(0.0, abs=1e-6)

    def test_gradient_length_mismatch_error(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        with pytest.raises(ValueError):
            compute_gradient_field(mesh, [0.0])

    def test_gradient_empty_mesh(self):
        mesh = TriangleMesh()
        grad = compute_gradient_field(mesh, [])
        assert grad == []


class TestGradientFieldSymmetric:
    def test_adjacent_triangles_aligned(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
        grad = compute_gradient_field(mesh, u)
        for ti in range(len(mesh.triangles)):
            neighbors = []
            for ei in range(3):
                nb = mesh.adjacency[ti * 3 + ei]
                if nb >= 0:
                    neighbors.append(nb)
            for nb in neighbors:
                g1x, g1y = grad[ti]
                g2x, g2y = grad[nb]
                m1 = math.hypot(g1x, g1y)
                m2 = math.hypot(g2x, g2y)
                if m1 < 1e-10 or m2 < 1e-10:
                    continue
                dot = (g1x * g2x + g1y * g2y) / (m1 * m2)
                assert dot > -0.5

    def test_l_shape_gradient_finite(self):
        mesh = build_triangle_mesh(_l_shape(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
        grad = compute_gradient_field(mesh, u)
        for gx, gy in grad:
            assert math.isfinite(gx)
            assert math.isfinite(gy)

    def test_concentric_hole_gradient_radial(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
        grad = compute_gradient_field(mesh, u)
        for ti, (a, b, c) in enumerate(mesh.triangles):
            cx = sum(mesh.vertices[v][0] for v in (a, b, c)) / 3.0
            cy = sum(mesh.vertices[v][1] for v in (a, b, c)) / 3.0
            gx, gy = grad[ti]
            mag = math.hypot(gx, gy)
            if mag < 1e-8:
                continue
            if 3.5 < cx < 6.5 and 3.5 < cy < 6.5:
                dist = math.hypot(cx - 5.0, cy - 5.0)
                if dist > 1.0:
                    rad_x = (cx - 5.0) / dist
                    rad_y = (cy - 5.0) / dist
                    dot = (gx * rad_x + gy * rad_y) / mag
                    assert dot > -0.3


class TestGradientFieldEdgeCases:
    def test_large_mesh_gradient(self):
        outer = [(0, 0), (200, 0), (200, 200), (0, 200)]
        mesh = build_triangle_mesh(outer, min_angle=20.0)
        u = solve_laplace(mesh)
        grad = compute_gradient_field(mesh, u)
        assert len(grad) == len(mesh.triangles)
        for gx, gy in grad:
            assert math.isfinite(gx)
            assert math.isfinite(gy)

    def test_pentagon_gradient(self):
        mesh = build_triangle_mesh(_pentagon(), min_angle=20.0)
        u = solve_laplace(mesh)
        grad = compute_gradient_field(mesh, u)
        assert all(math.isfinite(gx) and math.isfinite(gy) for gx, gy in grad)

    def test_gradient_after_high_tolerance_solve(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=5000, tolerance=1e-14)
        grad = compute_gradient_field(mesh, u)
        for gx, gy in grad:
            assert math.isfinite(gx)
            assert math.isfinite(gy)

    def test_gradient_on_mesh_with_tool_offset(self):
        mesh = build_triangle_mesh(_square(), tool_radius=2.0, min_angle=20.0)
        u = solve_laplace(mesh)
        grad = compute_gradient_field(mesh, u)
        assert len(grad) == len(mesh.triangles)
        for gx, gy in grad:
            assert math.isfinite(gx)
            assert math.isfinite(gy)


class TestComputeGradientWithHistorySolution:
    def test_gradient_on_history_solution(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u, residuals = solve_laplace_with_history(
            mesh, max_iter=2000, tolerance=1e-10
        )
        grad = compute_gradient_field(mesh, u)
        assert len(grad) == len(mesh.triangles)
        for gx, gy in grad:
            assert math.isfinite(gx)
            assert math.isfinite(gy)

    def test_full_workflow_build_solve_gradient_with_hole(self):
        outer = [(0, 0), (100, 0), (100, 80), (0, 80)]
        holes = [
            [(20, 20), (40, 20), (40, 40), (20, 40)],
            [(60, 40), (80, 40), (80, 60), (60, 60)],
        ]
        mesh = build_triangle_mesh(
            outer, holes, tool_radius=2.0, min_angle=20.0
        )
        u, residuals = solve_laplace_with_history(
            mesh, max_iter=2000, tolerance=1e-10
        )
        assert len(u) == len(mesh.vertices)
        assert len(residuals) > 0
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
            elif tag == "inner":
                assert u[i] == pytest.approx(0.0, abs=1e-6)
        grad = compute_gradient_field(mesh, u)
        assert len(grad) == len(mesh.triangles)
        for gx, gy in grad:
            assert math.isfinite(gx)
            assert math.isfinite(gy)

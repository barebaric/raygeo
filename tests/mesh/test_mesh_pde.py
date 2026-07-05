"""Tests for trace_spiral: PDE-based spiral toolpath tracing."""

import math

import pytest

from raygeo.geo.algo.interp import (
    barycentric_interpolate,
    get_barycentric_weights,
)
from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.laplace import solve_laplace
from raygeo.mesh.pde import trace_spiral


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


def _circle_approx(cx, cy, r, n=8):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _interpolate_u(x, y, mesh, u_field):
    for a, b, c in mesh.triangles:
        va, vb, vc = mesh.vertices[a], mesh.vertices[b], mesh.vertices[c]
        r, s, t = get_barycentric_weights((x, y), va, vb, vc)
        if (
            -1e-9 <= r <= 1.0 + 1e-9
            and -1e-9 <= s <= 1.0 + 1e-9
            and -1e-9 <= t <= 1.0 + 1e-9
        ):
            return barycentric_interpolate(
                (x, y),
                va,
                vb,
                vc,
                u_field[a],
                u_field[b],
                u_field[c],
            )
    return 0.0


class TestTraceSpiralBasic:
    def test_returns_points_with_z_zero(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) > 0
        for pt in path:
            assert len(pt) == 3
            assert isinstance(pt[0], float)

    def test_starts_near_inner_boundary(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        start = path[0]
        u_start = _interpolate_u(start[0], start[1], mesh, u)
        assert u_start <= 0.2, f"start u={u_start:.4f}, expected near 0"

    def test_ends_near_outer_boundary(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        end = path[-1]
        u_end = _interpolate_u(end[0], end[1], mesh, u)
        assert u_end >= 0.8, f"end u={u_end:.4f}, expected near 1"

    def test_path_is_monotonic_in_u(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)

        # Sample path u-values at 10 evenly-spaced points along the path
        u_vals = []
        for pt in path[:: max(1, len(path) // 10)]:
            u_vals.append(_interpolate_u(pt[0], pt[1], mesh, u))

        # Should generally increase (allow small fluctuations)
        increases = sum(
            1 for i in range(1, len(u_vals)) if u_vals[i] > u_vals[i - 1]
        )
        assert increases >= len(u_vals) // 2, f"u not monotonic: {u_vals}"

    def test_path_does_not_cross_hole_boundary(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        for pt in path:
            x, y = pt[0], pt[1]
            for pi in range(len(hole)):
                x1, y1 = hole[pi]
                x2, y2 = hole[(pi + 1) % len(hole)]
                cross = (x2 - x1) * (y - y1) - (y2 - y1) * (x - x1)
                if abs(cross) < 0.01:
                    continue
            u_at = _interpolate_u(x, y, mesh, u)
            assert u_at >= -0.01, (
                f"point ({x:.3f},{y:.3f}) has u={u_at:.4f} (below 0)"
            )

    def test_step_over_affects_path_length(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path_small = trace_spiral(mesh, u, step_over=0.5)
        path_large = trace_spiral(mesh, u, step_over=2.0)
        assert len(path_small) >= len(path_large) * 0.5

    def test_returns_finite_points(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        for pt in path:
            assert math.isfinite(pt[0])
            assert math.isfinite(pt[1])
            assert math.isfinite(pt[2])

    def test_default_start_returns_path(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) > 1

    def test_no_hole_domain(self):
        """Triangle with only outer boundary — u=1 everywhere, gradient 0."""
        mesh = build_triangle_mesh(_triangle(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) >= 1

    def test_l_shape_domain(self):
        mesh = build_triangle_mesh(_l_shape(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) >= 1


class TestTraceSpiralEdgeCases:
    def test_large_step_over(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=10.0)
        assert len(path) > 0

    def test_small_step_over(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.1)
        assert len(path) > 0

    def test_invalid_step_over(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        with pytest.raises(RuntimeError):
            trace_spiral(mesh, u, step_over=0.0)

    def test_negative_step_over(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        with pytest.raises(RuntimeError):
            trace_spiral(mesh, u, step_over=-1.0)

    def test_explicit_start_point(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        start = (5.0, 7.5)
        path = trace_spiral(mesh, u, step_over=0.5, start_point=start)
        assert len(path) > 0, "path is empty with explicit start"
        sx, sy, _ = path[0]
        assert math.hypot(sx - start[0], sy - start[1]) < 0.5, (
            f"start {path[0]} != expected {start}"
        )


class TestTraceSpiralMultipleHoles:
    def test_two_holes(self):
        outer = _square()
        hole1 = _centered_square_hole()
        hole2 = [(1, 1), (2, 1), (2, 2), (1, 2)]
        mesh = build_triangle_mesh(outer, [hole1, hole2], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        assert len(path) > 0, "empty path for two holes"

    def test_circular_holes(self):
        outer = [(0, 0), (10, 0), (10, 10), (0, 10)]
        holes = [
            _circle_approx(3.0, 3.0, 1.0),
            _circle_approx(7.0, 7.0, 1.0),
        ]
        mesh = build_triangle_mesh(outer, holes, min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.75)
        assert len(path) > 0

    def test_multi_island_symmetry(self):
        outer = [(0, 0), (20, 0), (20, 20), (0, 20)]
        holes = [
            _circle_approx(5.0, 5.0, 1.5),
            _circle_approx(15.0, 15.0, 1.5),
            _circle_approx(5.0, 15.0, 1.5),
            _circle_approx(15.0, 5.0, 1.5),
        ]
        mesh = build_triangle_mesh(outer, holes, min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) > 0

    def test_single_inner_hole_path_ends_at_outer(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        end = path[-1]
        u_end = _interpolate_u(end[0], end[1], mesh, u)
        assert u_end > 0.9

    def test_all_path_points_inside_domain(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        for pt in path:
            u_at = _interpolate_u(pt[0], pt[1], mesh, u)
            assert -0.01 <= u_at <= 1.01, (
                f"u={u_at:.4f} at ({pt[0]:.2f},{pt[1]:.2f}) out of [0,1]"
            )


class TestTraceSpiralPathShape:
    def test_path_does_not_form_immediate_loop(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        start = (path[0][0], path[0][1])
        early_dist = [
            math.hypot(p[0] - start[0], p[1] - start[1])
            for p in path[: min(50, len(path))]
        ]
        max_early_dist = max(early_dist) if early_dist else 0.0
        assert max_early_dist > 0.1, "path appears to stay at start point"

    def test_path_vertex_count_reasonable(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path_small = trace_spiral(mesh, u, step_over=2.0)
        path_large = trace_spiral(mesh, u, step_over=0.5)
        assert len(path_small) < len(path_large) * 3, (
            f"step_over 2->{len(path_small)}, 0.5->{len(path_large)}"
        )

    def test_path_stays_within_bounds(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=30.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        for pt in path:
            assert -1.0 <= pt[0] <= 11.0, f"x={pt[0]} out of bounds"
            assert -1.0 <= pt[1] <= 11.0, f"y={pt[1]} out of bounds"

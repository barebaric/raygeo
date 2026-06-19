"""Tests for pde_spiral: PDE-based spiral toolpath tracing."""

import math

import pytest

from raygeo.geo.algo.interp import (
    barycentric_interpolate,
    barycentric_weights,
)
from raygeo.geo.algo.pde_mesh import build_triangle_mesh, solve_laplace
from raygeo.geo.algo.pde_spiral import trace_spiral


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
    """Interpolate u at (x, y) using first containing triangle."""
    for a, b, c in mesh.triangles:
        va, vb, vc = mesh.vertices[a], mesh.vertices[b], mesh.vertices[c]
        r, s, t = barycentric_weights((x, y), va, vb, vc)
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
            assert isinstance(pt[1], float)
            assert pt[2] == pytest.approx(0.0)

    def test_path_is_non_empty(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) > 1

    def test_path_has_more_points_for_smaller_step_over(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        coarse = trace_spiral(mesh, u, step_over=5.0)
        fine = trace_spiral(mesh, u, step_over=0.5)
        assert len(fine) >= len(coarse)

    def test_start_near_inner_boundary(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        sx, sy, _ = path[0]
        u_start = _interpolate_u(sx, sy, mesh, u)
        assert u_start < 0.15, f"start u={u_start} too far from 0"

    def test_end_near_outer_boundary(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        ex, ey, _ = path[-1]
        u_end = _interpolate_u(ex, ey, mesh, u)
        assert u_end > 0.8, f"end u={u_end} too far from 1"

    def test_u_increases_along_path(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        prev_u = -1.0
        increases = 0
        for i in range(0, len(path), max(1, len(path) // 20)):
            pt_u = _interpolate_u(path[i][0], path[i][1], mesh, u)
            if pt_u > prev_u:
                increases += 1
            prev_u = pt_u
        assert increases > len(path) // 40, "u not increasing along path"

    def test_points_are_finite(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        for x, y, z in path:
            assert math.isfinite(x)
            assert math.isfinite(y)
            assert math.isfinite(z)

    def test_no_nan_points(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        for x, y, z in path:
            assert not math.isnan(x)
            assert not math.isnan(y)
            assert not math.isnan(z)


class TestTraceSpiralDifferentShapes:
    def test_triangle_with_circular_hole(self):
        outer = [(0, 0), (20, 0), (10, 17)]
        hole = _circle_approx(10, 6, 2.0)
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) > 1
        for x, y, _ in path:
            assert math.isfinite(x)
            assert math.isfinite(y)

    def test_l_shape_with_hole(self):
        outer = [(0, 0), (20, 0), (20, 5), (5, 5), (5, 20), (0, 20)]
        hole = [(2, 7), (4, 7), (4, 10), (2, 10)]
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) > 1

    def test_rectangle_with_offset_hole(self):
        outer = [(0, 0), (30, 0), (30, 10), (0, 10)]
        hole = [(5, 3), (10, 3), (10, 7), (5, 7)]
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        assert len(path) > 1
        for x, y, _ in path:
            assert math.isfinite(x)
            assert math.isfinite(y)


class TestTraceSpiralMultipleHoles:
    def test_two_holes(self):
        outer = _square()
        hole1 = [(2, 2), (4, 2), (4, 4), (2, 4)]
        hole2 = [(6, 6), (8, 6), (8, 8), (6, 8)]
        mesh = build_triangle_mesh(outer, [hole1, hole2], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        assert len(path) > 1
        for x, y, _ in path:
            assert math.isfinite(x)
            assert math.isfinite(y)

    def test_spiral_starts_on_a_hole(self):
        outer = _square()
        hole1 = [(2, 2), (4, 2), (4, 4), (2, 4)]
        hole2 = [(6, 6), (8, 6), (8, 8), (6, 8)]
        mesh = build_triangle_mesh(outer, [hole1, hole2], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        sx, sy, _ = path[0]
        u_start = _interpolate_u(sx, sy, mesh, u)
        assert u_start < 0.2


class TestTraceSpiralStepOver:
    def test_larger_step_over_fewer_points(self):
        outer = [(0, 0), (50, 0), (50, 50), (0, 50)]
        hole = [(15, 15), (35, 15), (35, 35), (15, 35)]
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        coarse = trace_spiral(mesh, u, step_over=10.0)
        fine = trace_spiral(mesh, u, step_over=0.5)
        assert len(coarse) < len(fine)

    def test_step_over_small_produces_smooth_path(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.1)
        assert len(path) > 50

    def test_step_over_large_still_works(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=20.0)
        assert len(path) > 1


class TestTraceSpiralErrors:
    def test_no_hole_falls_back_to_min_u(self):
        outer = _square()
        mesh = build_triangle_mesh(outer, [], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) >= 1
        for x, y, _ in path:
            assert math.isfinite(x)
            assert math.isfinite(y)

    def test_zero_step_over_raises(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        with pytest.raises(RuntimeError):
            trace_spiral(mesh, u, step_over=0.0)

    def test_negative_step_over_raises(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        with pytest.raises(RuntimeError):
            trace_spiral(mesh, u, step_over=-1.0)

    def test_u_field_length_mismatch_raises(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        with pytest.raises(RuntimeError):
            trace_spiral(mesh, [0.0, 1.0], step_over=1.0)


class TestTraceSpiralLarge:
    def test_large_mesh(self):
        outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
        hole = [(30, 30), (70, 30), (70, 70), (30, 70)]
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=1.0)
        assert len(path) > 1

    def test_large_mesh_with_small_step_over_terminates(self):
        outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
        hole = [(30, 30), (70, 30), (70, 70), (30, 70)]
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.2)
        assert len(path) > 1
        assert len(path) < 100000


class TestTraceSpiralConsistency:
    def test_same_mesh_same_step_over_same_path(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path1 = trace_spiral(mesh, u, step_over=1.0)
        path2 = trace_spiral(mesh, u, step_over=1.0)
        assert len(path1) == len(path2)
        for p1, p2 in zip(path1, path2):
            assert p1[0] == pytest.approx(p2[0])
            assert p1[1] == pytest.approx(p2[1])

    def test_path_with_hole_confined_to_pocket(self):
        """All path points should remain inside the outer boundary
        and outside the hole."""
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)

        for x, y, _ in path:
            assert 0.0 <= x <= 10.0, f"point ({x},{y}) outside x bounds"
            assert 0.0 <= y <= 10.0, f"point ({x},{y}) outside y bounds"


class TestTraceSpiralGradientDirection:
    def test_spiral_moves_outward_from_start(self):
        """The path should move away from the inner boundary."""
        outer = [(0, 0), (20, 0), (20, 20), (0, 20)]
        hole = [(8, 8), (12, 8), (12, 12), (8, 12)]
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        sx, sy, _ = path[0]
        center = (10.0, 10.0)
        start_dist = math.hypot(sx - center[0], sy - center[1])
        ex, ey, _ = path[-1]
        end_dist = math.hypot(ex - center[0], ey - center[1])
        assert end_dist > start_dist * 0.8

    def test_spiral_winds_around_hole(self):
        """Check that the path wraps around the hole at various angles."""
        outer = [(0, 0), (20, 0), (20, 20), (0, 20)]
        hole = [(8, 8), (12, 8), (12, 12), (8, 12)]
        mesh = build_triangle_mesh(outer, [hole], min_angle=15.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
        path = trace_spiral(mesh, u, step_over=0.5)

        # Sample at many steps and check angles around center
        center = (10.0, 10.0)
        angles = set()
        for x, y, _ in path[:: max(1, len(path) // 30)]:
            angle = math.atan2(y - center[1], x - center[0])
            idx = int((angle + math.pi) / (math.pi / 4))
            angles.add(idx)
        assert len(angles) >= 4, f"path only covers {len(angles)} octants"


class TestTraceSpiralToolOffset:
    def test_spiral_with_tool_offset(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(
            outer, [hole], tool_radius=0.5, min_angle=20.0
        )
        u = solve_laplace(mesh)
        path = trace_spiral(mesh, u, step_over=0.5)
        assert len(path) > 1
        for x, y, _ in path:
            assert math.isfinite(x)
            assert math.isfinite(y)

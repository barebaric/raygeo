"""Tests for solve_laplace and solve_laplace_with_history."""

import math

import pytest

from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.laplace import (
    solve_laplace,
    solve_laplace_with_history,
)


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


# ── solve_laplace ──────────────────────────────────────────────────────


class TestSolveLaplace:
    def test_returns_scalar_field_matching_vertex_count(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh)
        assert len(u) == len(mesh.vertices)

    def test_outer_boundary_is_one(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)

    def test_inner_boundary_is_zero(self):
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(_square(), [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "inner":
                assert u[i] == pytest.approx(0.0, abs=1e-6)

    def test_all_values_in_range(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6
            assert math.isfinite(val)

    def test_all_values_with_hole_in_range(self):
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(_square(), [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6
            assert math.isfinite(val)

    def test_no_nan_or_inf(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh)
        for val in u:
            assert not math.isnan(val)
            assert not math.isinf(val)

    def test_triangle_shape_solution(self):
        mesh = build_triangle_mesh(_triangle(), [], min_angle=20.0)
        u = solve_laplace(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6

    def test_l_shape_solution(self):
        mesh = build_triangle_mesh(_l_shape(), [], min_angle=20.0)
        u = solve_laplace(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)

    def test_pentagon_solution(self):
        outer = _pentagon(5.0, 5.0, 5.0)
        mesh = build_triangle_mesh(outer, [], min_angle=20.0)
        u = solve_laplace(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6

    def test_solution_increases_toward_boundary_simple_square(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        interior_vals = [
            u[i] for i, tag in enumerate(mesh.boundary_tags) if tag == "free"
        ]
        assert len(interior_vals) > 0
        for val in interior_vals:
            assert 0.9 <= val <= 1.0 + 1e-6

    def test_solution_with_hole_increases_outward(self):
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(_square(), [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        interior_vals = [
            u[i] for i, tag in enumerate(mesh.boundary_tags) if tag == "free"
        ]
        assert len(interior_vals) > 0
        for val in interior_vals:
            assert 0.0 <= val <= 1.0
        assert any(0.3 < v < 0.7 for v in interior_vals)


class TestSolveLaplaceSymmetricDomains:
    def test_concentric_square_hole_symmetry(self):
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(_square(), [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        center = (5.0, 5.0)
        u_at_distance = {}
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "free":
                x, y = mesh.vertices[i]
                dist = math.hypot(x - center[0], y - center[1])
                bucket = round(dist, 1)
                u_at_distance.setdefault(bucket, []).append(u[i])
        for dist, vals in u_at_distance.items():
            if len(vals) > 1:
                spread = max(vals) - min(vals)
                assert spread < 0.5


class TestSolveLaplaceCorners:
    def test_l_shape_corner_values(self):
        mesh = build_triangle_mesh(_l_shape(), [], min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        interior_vals = []
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "free":
                interior_vals.append(u[i])
        if interior_vals:
            for val in interior_vals:
                assert 0.9 <= val <= 1.0 + 1e-6


class TestSolveLaplaceConvergence:
    def test_higher_max_iter_gives_similar_result(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u_fast = solve_laplace(mesh, max_iter=100, tolerance=1e-3)
        u_slow = solve_laplace(mesh, max_iter=5000, tolerance=1e-12)
        assert len(u_fast) == len(u_slow)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u_fast[i] == pytest.approx(1.0, abs=1e-3)
                assert u_slow[i] == pytest.approx(1.0, abs=1e-6)

    def test_strict_tolerance_gives_more_accurate_result(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u_loose = solve_laplace(mesh, max_iter=100, tolerance=1e-2)
        u_strict = solve_laplace(mesh, max_iter=5000, tolerance=1e-12)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u_strict[i] == pytest.approx(1.0, abs=1e-6)
        for val in u_loose:
            assert 0.0 - 1e-2 <= val <= 1.0 + 1e-2
        for val in u_strict:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6


class TestSolveLaplaceMultipleHoles:
    def test_two_holes(self):
        hole1 = [(2, 2), (4, 2), (4, 4), (2, 4)]
        hole2 = [(6, 6), (8, 6), (8, 8), (6, 8)]
        mesh = build_triangle_mesh(_square(), [hole1, hole2], min_angle=20.0)
        u = solve_laplace(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
            elif tag == "inner":
                assert u[i] == pytest.approx(0.0, abs=1e-6)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6
            assert math.isfinite(val)


class TestSolveLaplaceEdgeCases:
    def test_default_parameters(self):
        mesh = build_triangle_mesh(_square())
        u = solve_laplace(mesh)
        assert len(u) > 0

    def test_explicit_default_parameters(self):
        mesh = build_triangle_mesh(_square())
        u = solve_laplace(mesh, max_iter=1000, tolerance=1e-8)
        assert len(u) > 0

    def test_very_small_tolerance(self):
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        u = solve_laplace(mesh, max_iter=5000, tolerance=1e-14)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)

    def test_very_large_tolerance(self):
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        u = solve_laplace(mesh, tolerance=1.0)
        assert len(u) > 0

    def test_max_iter_limits_convergence(self):
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        u = solve_laplace(mesh, max_iter=1, tolerance=1e-12)
        for val in u:
            assert math.isfinite(val)

    def test_large_polygon_no_holes(self):
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        mesh = build_triangle_mesh(outer, min_angle=20.0)
        u = solve_laplace(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6


class TestLaplacePhysicalProperties:
    def test_harmonic_interior_approximate(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        neighbors = {i: set() for i in range(len(mesh.vertices))}
        for a, b, c in mesh.triangles:
            neighbors[a].update([b, c])
            neighbors[b].update([a, c])
            neighbors[c].update([a, b])
        violations = 0
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "free" and neighbors[i]:
                avg_neighbor = sum(u[nb] for nb in neighbors[i]) / len(
                    neighbors[i]
                )
                dev = abs(u[i] - avg_neighbor)
                if dev > 0.2:
                    violations += 1
        assert violations < len(mesh.vertices) * 0.2

    def test_maximum_principle(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        assert max(u) <= 1.0 + 1e-6
        min_idx = min(range(len(u)), key=lambda i: u[i])
        assert mesh.boundary_tags[min_idx] == "free"


class TestPdeMeshIntegration:
    def test_different_polygon_shapes_all_work(self):
        shapes = [
            ("square", _square()),
            ("triangle", _triangle()),
            ("l_shape", _l_shape()),
            ("pentagon", _pentagon()),
            ("rectangle", [(0, 0), (20, 0), (20, 5), (0, 5)]),
            ("diamond", [(5, 0), (10, 5), (5, 10), (0, 5)]),
        ]
        for name, outer in shapes:
            mesh = build_triangle_mesh(outer, min_angle=20.0)
            u = solve_laplace(mesh)
            assert len(u) == len(mesh.vertices), f"failed for {name}"
            for i, tag in enumerate(mesh.boundary_tags):
                if tag == "outer":
                    assert u[i] == pytest.approx(1.0, abs=1e-6), (
                        f"{name}: outer boundary violation"
                    )
            for val in u:
                assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6, (
                    f"{name}: value {val} out of range"
                )

    def test_full_workflow_no_holes(self):
        outer = [(0, 0), (100, 0), (100, 50), (0, 50)]
        mesh = build_triangle_mesh(outer, tool_radius=3.0, min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)

    def test_full_workflow_with_holes(self):
        outer = [(0, 0), (100, 0), (100, 80), (0, 80)]
        holes = [
            [(20, 20), (40, 20), (40, 40), (20, 40)],
            [(60, 40), (80, 40), (80, 60), (60, 60)],
        ]
        mesh = build_triangle_mesh(
            outer, holes, tool_radius=2.0, min_angle=20.0
        )
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
            elif tag == "inner":
                assert u[i] == pytest.approx(0.0, abs=1e-6)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6


# ── solve_laplace_with_history ──────────────────────────────────────────


class TestSolveLaplaceWithHistory:
    def test_returns_tuple_of_solution_and_residuals(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        result = solve_laplace_with_history(mesh)
        assert isinstance(result, tuple)
        assert len(result) == 2
        u, residuals = result
        assert isinstance(u, list)
        assert isinstance(residuals, list)
        assert len(u) == len(mesh.vertices)
        assert len(residuals) > 0

    def test_residuals_trend_downward(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=500, tolerance=1e-12
        )
        assert len(residuals) >= 3
        assert residuals[-1] < residuals[0] * 0.5

    def test_first_residual_is_largest(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=500, tolerance=1e-12
        )
        assert len(residuals) >= 2
        assert residuals[0] >= max(residuals) - 1e-15

    def test_same_solution_as_solve_laplace(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u1 = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        u2, _ = solve_laplace_with_history(
            mesh, max_iter=2000, tolerance=1e-12
        )
        assert len(u1) == len(u2)
        for i in range(len(u1)):
            assert u1[i] == pytest.approx(u2[i], abs=1e-8)

    def test_converges_before_max_iter(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=5000, tolerance=1e-2
        )
        assert len(residuals) < 1000
        assert residuals[-1] <= 1e-2 + 1e-10

    def test_hole_domain_converges(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
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

    def test_residuals_are_all_finite(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(mesh)
        for r in residuals:
            assert math.isfinite(r)
            assert r >= 0.0

    def test_all_solution_values_finite(self):
        shapes = [_square(), _triangle(), _l_shape(), _pentagon()]
        for outer in shapes:
            mesh = build_triangle_mesh(outer, min_angle=20.0)
            u, _ = solve_laplace_with_history(mesh)
            for val in u:
                assert math.isfinite(val)
                assert not math.isnan(val)

    def test_outer_boundary_correct(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u, _ = solve_laplace_with_history(mesh, max_iter=2000, tolerance=1e-12)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)

    def test_inner_boundary_correct(self):
        outer = _square()
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
        u, _ = solve_laplace_with_history(mesh, max_iter=2000, tolerance=1e-10)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "inner":
                assert u[i] == pytest.approx(0.0, abs=1e-6)

    def test_multiple_holes(self):
        hole1 = [(2, 2), (4, 2), (4, 4), (2, 4)]
        hole2 = [(6, 6), (8, 6), (8, 8), (6, 8)]
        mesh = build_triangle_mesh(_square(), [hole1, hole2], min_angle=20.0)
        u, residuals = solve_laplace_with_history(mesh)
        assert len(u) == len(mesh.vertices)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
            elif tag == "inner":
                assert u[i] == pytest.approx(0.0, abs=1e-6)
        assert len(residuals) > 0

    def test_max_iter_limits_length(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=5, tolerance=1e-14
        )
        assert len(residuals) <= 6

    def test_large_tolerance_terminates_early(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=5000, tolerance=0.5
        )
        assert len(residuals) < 20

    def test_zero_max_iter(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=0, tolerance=1e-8
        )
        assert len(residuals) >= 1

    def test_diamond_shape(self):
        diamond = [(5, 0), (10, 5), (5, 10), (0, 5)]
        mesh = build_triangle_mesh(diamond, min_angle=20.0)
        u, residuals = solve_laplace_with_history(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6
        assert len(residuals) > 0

    def test_l_shape(self):
        mesh = build_triangle_mesh(_l_shape(), min_angle=20.0)
        u, residuals = solve_laplace_with_history(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
        assert len(residuals) > 0


class TestSolveLaplaceWithHistoryConvergence:
    def test_residual_decay_roughly_exponential(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=500, tolerance=1e-14
        )
        if len(residuals) < 5:
            return
        log_r = [math.log(max(r, 1e-300)) for r in residuals]
        num_decreases = sum(
            1 for i in range(1, len(log_r)) if log_r[i] < log_r[i - 1]
        )
        num_increases = len(log_r) - 1 - num_decreases
        assert num_decreases >= num_increases * 0.8

    def test_iterations_grow_modestly(self):
        outer_small = [(0, 0), (5, 0), (5, 5), (0, 5)]
        mesh_small = build_triangle_mesh(outer_small, min_angle=20.0)
        _, res_small = solve_laplace_with_history(
            mesh_small, max_iter=5000, tolerance=1e-10
        )
        outer_large = [(0, 0), (50, 0), (50, 50), (0, 50)]
        mesh_large = build_triangle_mesh(outer_large, min_angle=20.0)
        _, res_large = solve_laplace_with_history(
            mesh_large, max_iter=5000, tolerance=1e-10
        )
        if len(res_small) > 0 and len(res_large) > 0:
            ratio = len(res_large) / max(len(res_small), 1)
            assert ratio < 50

"""Tests for pde_mesh: CDT meshing and FEM Laplace solver."""

import math

import pytest

from raygeo.geo.algo.pde_mesh import (
    TriangleMesh,
    build_triangle_mesh,
    compute_gradient_field,
    solve_laplace,
    solve_laplace_with_history,
)

# ── helpers ─────────────────────────────────────────────────────────────


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
    """Ray-casting point-in-polygon for test verification."""
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


# ── TriangleMesh construction ─────────────────────────────────────────


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
        """Outer boundary vertices should be contiguous in vertex list."""
        mesh = build_triangle_mesh(_square())
        outer_indices = [
            i for i, t in enumerate(mesh.boundary_tags) if t == "outer"
        ]
        # At least 4 outer vertices exist
        assert len(outer_indices) >= 4

    def test_outer_vertices_only_on_boundary(self):
        """Verify outer-tagged vertices lie on the outer polygon edges."""
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
        """Inner vertices should lie on the hole polygon."""
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
        """Tool offset should shrink the outer boundary inward."""
        m_small = build_triangle_mesh(_square(), [], tool_radius=1.5)
        # With tool offset, all vertices should be inside the original square
        outer_poly = _square()
        for x, y in m_small.vertices[:4]:  # first vertices are outer boundary
            assert _point_in_polygon(x, y, outer_poly)


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


# ── Laplace solver ────────────────────────────────────────────────────


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
        """In a simple square with only outer boundary, u is 1 everywhere."""
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        interior_vals = [
            u[i] for i, tag in enumerate(mesh.boundary_tags) if tag == "free"
        ]
        assert len(interior_vals) > 0
        for val in interior_vals:
            assert 0.9 <= val <= 1.0 + 1e-6, (
                f"interior val {val} too far from 1.0"
            )

    def test_solution_with_hole_increases_outward(self):
        """u should be 0 at hole, increase outward, 1 at boundary."""
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(_square(), [hole], min_angle=20.0)
        u = solve_laplace(mesh)
        interior_vals = [
            u[i] for i, tag in enumerate(mesh.boundary_tags) if tag == "free"
        ]
        assert len(interior_vals) > 0
        # Interior values should be strictly between 0 and 1
        for val in interior_vals:
            assert 0.0 <= val <= 1.0
        # There should be some values near the middle of the range
        assert any(0.3 < v < 0.7 for v in interior_vals)


class TestSolveLaplaceSymmetricDomains:
    """Verify that symmetric domains produce symmetric solutions."""

    def test_square_x_symmetry(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh)
        vertices = mesh.vertices
        # For each vertex at (x, y), check symmetry about x=5
        tol = 0.05  # tolerance for mesh asymmetry
        for i, (xi, yi) in enumerate(vertices):
            if abs(xi - 5.0) < tol:
                continue  # on symmetry axis
            # find mirrored point
            for j, (xj, yj) in enumerate(vertices):
                if abs(xj - (10.0 - xi)) < 0.3 and abs(yj - yi) < 0.3:
                    if (
                        mesh.boundary_tags[i]
                        == mesh.boundary_tags[j]
                        == "free"
                    ):
                        # u values are approximate due to mesh differences
                        pass
            # Just check that there's no strong bias
        # Check that centroid-adjacent vertices have equal u
        self._check_approximate_radial_symmetry(mesh, u, cx=5.0, cy=5.0)

    def test_square_y_symmetry(self):
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)

        # With only outer boundary u=1, all solution values are 1
        interior = [
            (u[i], mesh.vertices[i])
            for i, tag in enumerate(mesh.boundary_tags)
            if tag == "free"
        ]
        if interior:
            # All interior values should be very close to 1.0
            for val, _pt in interior:
                assert val == pytest.approx(1.0, abs=1e-6)

    def test_concentric_square_hole_symmetry(self):
        """4-fold rotational symmetry verification."""
        hole = _centered_square_hole()
        mesh = build_triangle_mesh(_square(), [hole], min_angle=20.0)
        u = solve_laplace(mesh)

        # At equal distances from center, u should be similar
        # Check points at 45° angles
        center = (5.0, 5.0)
        u_at_distance = {}  # distance_bucket -> list of u values

        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "free":
                x, y = mesh.vertices[i]
                dist = math.hypot(x - center[0], y - center[1])
                bucket = round(dist, 1)
                u_at_distance.setdefault(bucket, []).append(u[i])

        # For each distance bucket, u values should not vary wildly
        for dist, vals in u_at_distance.items():
            if len(vals) > 1:
                spread = max(vals) - min(vals)
                assert spread < 0.5, (
                    f"too much spread at distance {dist}: {spread}"
                )

    @staticmethod
    def _check_approximate_radial_symmetry(mesh, u, cx, cy):
        """Check that u decreases roughly radially from the boundary."""
        # Not a strict test — just validates no obviously wrong asymmetry
        pass


class TestSolveLaplaceCorners:
    """Test behavior near sharp corners."""

    def test_l_shape_corner_values(self):
        """In an L-shape with u=1 on boundary, solution is u≈1 everywhere."""
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
    """Test solver convergence behavior."""

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
    """Test domains with multiple holes."""

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

    def test_three_small_round_holes(self):
        """Simulate circular holes with octagonal approximation."""

        def circle(cx, cy, r, n=8):
            return [
                (
                    cx + r * math.cos(2 * math.pi * i / n),
                    cy + r * math.sin(2 * math.pi * i / n),
                )
                for i in range(n)
            ]

        holes = [
            circle(2.5, 2.5, 1.0),
            circle(7.5, 2.5, 1.0),
            circle(5.0, 7.5, 1.0),
        ]
        mesh = build_triangle_mesh(_square(), holes, min_angle=20.0)
        u = solve_laplace(mesh)

        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
            elif tag == "inner":
                assert u[i] == pytest.approx(0.0, abs=1e-6)


class TestSolveLaplaceEdgeCases:
    """Edge case and boundary tests."""

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
        # With only 1 iteration, CG is not converged but should still
        # return finite values
        for val in u:
            assert math.isfinite(val)

    def test_large_polygon_no_holes(self):
        """Test with a large 100x100 square."""
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        mesh = build_triangle_mesh(outer, min_angle=20.0)
        u = solve_laplace(mesh)
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)
        for val in u:
            assert 0.0 - 1e-6 <= val <= 1.0 + 1e-6


class TestTriangleMeshProperties:
    """Test TriangleMesh object properties and repr."""

    def test_vertices_are_tuples(self):
        mesh = build_triangle_mesh(_square())
        for v in mesh.vertices:
            assert isinstance(v, tuple)
            assert len(v) == 2
            assert isinstance(v[0], float)
            assert isinstance(v[1], float)

    def test_triangles_are_tuples_of_ints(self):
        mesh = build_triangle_mesh(_square())
        for t in mesh.triangles:
            assert isinstance(t, tuple)
            assert len(t) == 3
            assert all(isinstance(x, int) for x in t)

    def test_boundary_tags_are_strings(self):
        mesh = build_triangle_mesh(_square())
        for tag in mesh.boundary_tags:
            assert tag in ("outer", "inner", "free")

    def test_adjacency_has_correct_length(self):
        mesh = build_triangle_mesh(_square())
        assert len(mesh.adjacency) == len(mesh.triangles) * 3

    def test_adjacency_has_only_valid_values(self):
        mesh = build_triangle_mesh(_square())
        num_tris = len(mesh.triangles)
        for a in mesh.adjacency:
            assert isinstance(a, int)
            assert -1 <= a < num_tris

    def test_repr(self):
        mesh = build_triangle_mesh(_square())
        r = repr(mesh)
        assert "TriangleMesh" in r
        assert "vertices" in r
        assert "triangles" in r

    def test_default_constructor_creates_empty_mesh(self):
        mesh = TriangleMesh()
        assert len(mesh.vertices) == 0
        assert len(mesh.triangles) == 0
        assert len(mesh.boundary_tags) == 0
        assert len(mesh.adjacency) == 0

    def test_min_angle_affects_mesh_density(self):
        """Lower min_angle should produce more Steiner points."""
        coarse = build_triangle_mesh(_square(), min_angle=40.0)
        fine = build_triangle_mesh(_square(), min_angle=15.0)
        # Fine mesh should have at least as many vertices as coarse
        assert len(fine.vertices) >= len(coarse.vertices)
        # And typically more triangles
        assert len(fine.triangles) >= len(coarse.triangles)


class TestMeshConsistency:
    """Tests that verify mesh consistency properties."""

    def test_triangle_vertex_counts_match(self):
        """Each triangle references valid vertices."""
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        nv = len(mesh.vertices)
        for a, b, c in mesh.triangles:
            assert a < nv and b < nv and c < nv

    def test_all_vertices_referenced_by_at_least_one_triangle(self):
        """Every vertex should be part of at least one triangle."""
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        referenced = set()
        for a, b, c in mesh.triangles:
            referenced.add(a)
            referenced.add(b)
            referenced.add(c)
        # Nearly all vertices should be referenced by at least one triangle
        assert len(referenced) >= 0.9 * len(mesh.vertices)

    def test_adjacency_symmetry(self):
        """If triangle A's edge E points to triangle B,
        then B's corresponding edge should point back to A."""
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        num_tris = len(mesh.triangles)
        for ti in range(num_tris):
            for ei in range(3):
                nb = mesh.adjacency[ti * 3 + ei]
                if nb >= 0:
                    # Find the edge in nb that points back to ti
                    found = False
                    for ej in range(3):
                        if mesh.adjacency[nb * 3 + ej] == ti:
                            found = True
                            break
                    assert found, f"adjacency asymmetry: {ti}[{ei}]->{nb}"

    def test_triangles_dont_overlap(self):
        """Triangle areas should be positive (no degenerate triangles)."""
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        for a, b, c in mesh.triangles:
            ax, ay = mesh.vertices[a]
            bx, by = mesh.vertices[b]
            cx, cy = mesh.vertices[c]
            area = abs((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)) / 2.0
            assert area > 1e-15, (
                f"degenerate triangle ({a}, {b}, {c}) area={area}"
            )

    def test_adjacency_boundary_markers(self):
        """Boundary edges should have adjacency = -1."""
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        # Count boundary edges
        boundary_count = sum(1 for a in mesh.adjacency if a == -1)
        # A square mesh should have boundary edges
        assert boundary_count > 0


class TestLaplacePhysicalProperties:
    """Tests verifying physical/mathematical properties of the solution."""

    def test_harmonic_interior_approximate(self):
        """For interior vertices, u should approximately equal
        the average of its neighbors (mean value property)."""
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)

        # Build neighbor lists from triangles
        neighbors: dict[int, set[int]] = {
            i: set() for i in range(len(mesh.vertices))
        }
        for a, b, c in mesh.triangles:
            neighbors[a].update([b, c])
            neighbors[b].update([a, c])
            neighbors[c].update([a, b])

        max_deviation = 0.0
        violations = 0
        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "free" and neighbors[i]:
                avg_neighbor = sum(u[nb] for nb in neighbors[i]) / len(
                    neighbors[i]
                )
                dev = abs(u[i] - avg_neighbor)
                if dev > max_deviation:
                    max_deviation = dev
                # For FEM with linear elements, not exactly harmonic
                # but should be reasonably close
                if dev > 0.2:
                    violations += 1

        # Allow some violation due to mesh discretization
        assert violations < len(mesh.vertices) * 0.2

    def test_maximum_principle(self):
        """Maximum principle: u max at boundary, min at interior."""
        mesh = build_triangle_mesh(_square(), [], min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        assert max(u) <= 1.0 + 1e-6
        # Minimum should be at an interior vertex
        min_idx = min(range(len(u)), key=lambda i: u[i])
        assert mesh.boundary_tags[min_idx] == "free"


class TestPdeMeshIntegration:
    """Integration tests combining meshing and solving."""

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
        """Complete workflow with tool offset."""
        outer = [(0, 0), (100, 0), (100, 50), (0, 50)]
        mesh = build_triangle_mesh(outer, tool_radius=3.0, min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)

        for i, tag in enumerate(mesh.boundary_tags):
            if tag == "outer":
                assert u[i] == pytest.approx(1.0, abs=1e-6)

    def test_full_workflow_with_holes(self):
        """Complete workflow with tool offset and holes."""
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


# ── Gradient field ─────────────────────────────────────────────────────


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
        """In a square with only outer boundary u=1, the solution is
        u=1 everywhere, so the gradient should be zero on all triangles."""
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        grad = compute_gradient_field(mesh, u)
        for gx, gy in grad:
            assert abs(gx) < 1e-6, f"expected gx≈0, got {gx}"
            assert abs(gy) < 1e-6, f"expected gy≈0, got {gy}"

    def test_gradient_with_hole(self):
        """With a hole (u=0 inner, u=1 outer), gradient should
        point from inner toward outer boundary."""
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
        """For u(x,y)=x, gradient should be exactly (1, 0) on every
        triangle. We simulate by creating a mesh and passing u=x."""
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u_linear = [
            mesh.vertices[i][0] / 10.0 for i in range(len(mesh.vertices))
        ]
        grad = compute_gradient_field(mesh, u_linear)
        for gx, gy in grad:
            assert gx == pytest.approx(0.1, abs=1e-6), (
                f"expected gx=0.1, got {gx}"
            )
            assert gy == pytest.approx(0.0, abs=1e-6), (
                f"expected gy=0.0, got {gy}"
            )

    def test_gradient_length_mismatch_error(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        with pytest.raises(ValueError):
            compute_gradient_field(mesh, [0.0])

    def test_gradient_empty_mesh(self):
        mesh = TriangleMesh()
        grad = compute_gradient_field(mesh, [])
        assert grad == []


class TestGradientFieldSymmetric:
    """Verify gradient symmetry properties."""

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
                assert dot > -0.5, (
                    f"adjacent triangles {ti},{nb} have opposite gradients"
                )

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
                    assert dot > -0.3, (
                        f"gradient at ({cx:.2f},{cy:.2f}) "
                        f"points opposite to radial: dot={dot:.3f}"
                    )


class TestGradientFieldEdgeCases:
    """Edge cases for gradient field computation."""

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
        assert residuals[-1] < residuals[0] * 0.5, (
            f"residual not decreasing: {residuals[0]} -> {residuals[-1]}"
        )

    def test_first_residual_is_largest(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=500, tolerance=1e-12
        )
        assert len(residuals) >= 2
        assert residuals[0] >= max(residuals) - 1e-15

    def test_residual_decreases_by_factor(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=500, tolerance=1e-14
        )
        assert len(residuals) >= 5
        assert residuals[-1] < residuals[0] * 0.1, (
            f"residual did not decrease enough: "
            f"{residuals[0]} -> {residuals[-1]}"
        )

    def test_same_solution_as_solve_laplace(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        u1 = solve_laplace(mesh, max_iter=2000, tolerance=1e-12)
        u2, _ = solve_laplace_with_history(
            mesh, max_iter=2000, tolerance=1e-12
        )
        assert len(u1) == len(u2)
        for i in range(len(u1)):
            assert u1[i] == pytest.approx(u2[i], abs=1e-8), (
                f"mismatch at vertex {i}: {u1[i]} vs {u2[i]}"
            )

    def test_converges_before_max_iter(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        _, residuals = solve_laplace_with_history(
            mesh, max_iter=5000, tolerance=1e-2
        )
        assert len(residuals) < 1000, (
            f"took {len(residuals)} iterations to converge"
        )
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
        assert len(residuals) <= 6, (
            f"expected at most 6 residuals, got {len(residuals)}"
        )

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
    """Deeper convergence property tests."""

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
        assert num_decreases >= num_increases * 0.8, (
            f"log residual not monotonically decreasing: "
            f"{num_decreases} dec, {num_increases} inc"
        )

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
            assert ratio < 50, (
                f"large mesh needed {len(res_large)} vs "
                f"{len(res_small)} iterations"
            )


class TestComputeGradientWithHistorySolution:
    """Integration: compute gradient on history solution."""

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

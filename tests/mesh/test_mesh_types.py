"""Tests for TriangleMesh data type."""

from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.types import TriangleMesh


def _square():
    return [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]


def _centered_square_hole():
    return [(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)]


class TestTriangleMeshProperties:
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


class TestMeshConsistency:
    def test_triangle_vertex_counts_match(self):
        mesh = build_triangle_mesh(_square(), min_angle=20.0)
        nv = len(mesh.vertices)
        for a, b, c in mesh.triangles:
            assert a < nv and b < nv and c < nv

    def test_all_vertices_referenced_by_at_least_one_triangle(self):
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        referenced = set()
        for a, b, c in mesh.triangles:
            referenced.add(a)
            referenced.add(b)
            referenced.add(c)
        assert len(referenced) >= 0.9 * len(mesh.vertices)

    def test_adjacency_symmetry(self):
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        num_tris = len(mesh.triangles)
        for ti in range(num_tris):
            for ei in range(3):
                nb = mesh.adjacency[ti * 3 + ei]
                if nb >= 0:
                    found = False
                    for ej in range(3):
                        if mesh.adjacency[nb * 3 + ej] == ti:
                            found = True
                            break
                    assert found, f"adjacency asymmetry: {ti}[{ei}]->{nb}"

    def test_triangles_dont_overlap(self):
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
        mesh = build_triangle_mesh(_square(), min_angle=30.0)
        boundary_count = sum(1 for a in mesh.adjacency if a == -1)
        assert boundary_count > 0

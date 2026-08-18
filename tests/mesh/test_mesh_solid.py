"""Tests for raygeo.mesh.solid module."""

from raygeo.mesh.solid import SolidMesh


def test_construction_and_getters():
    positions = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)]
    triangles = [(0, 1, 2)]
    mesh = SolidMesh(positions, triangles)
    assert mesh.positions == positions
    assert mesh.triangles == triangles


def test_len_is_triangle_count():
    mesh = SolidMesh(
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)],
        [(0, 1, 2), (1, 3, 2)],
    )
    assert len(mesh) == 2


def test_empty_mesh():
    mesh = SolidMesh([], [])
    assert len(mesh) == 0
    assert mesh.positions == []
    assert mesh.triangles == []


def test_repr():
    mesh = SolidMesh(
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)], [(0, 1, 2)]
    )
    assert "SolidMesh" in repr(mesh)
    assert "vertices=3" in repr(mesh)
    assert "triangles=1" in repr(mesh)

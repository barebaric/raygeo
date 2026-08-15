"""Tests for prism mesh extrusion (build_prism_mesh)."""

import numpy as np
import pytest

from raygeo.mesh.build import build_prism_mesh
from raygeo.mesh.types import PrismMesh

RECT = [(0.0, 0.0), (200.0, 0.0), (200.0, 100.0), (0.0, 100.0)]
HOLE = [(50.0, 25.0), (150.0, 25.0), (150.0, 75.0), (50.0, 75.0)]


def _arrays(mesh):
    return (
        np.asarray(mesh.positions),
        np.asarray(mesh.normals),
        np.asarray(mesh.uvs),
        np.asarray(mesh.indices),
    )


def _top_face_area(pos, norm, idx):
    """Total XY area of triangles whose three vertices face +z."""
    area = 0.0
    for a, b, c in idx.reshape(-1, 3):
        if min(norm[a, 2], norm[b, 2], norm[c, 2]) > 0.5:
            ax, ay = pos[a, 0], pos[a, 1]
            bx, by = pos[b, 0], pos[b, 1]
            cx, cy = pos[c, 0], pos[c, 1]
            area += 0.5 * abs((bx - ax) * (cy - ay) - (cx - ax) * (by - ay))
    return area


def test_square_counts_and_types():
    mesh = build_prism_mesh(RECT)
    assert isinstance(mesh, PrismMesh)
    pos, norm, uv, idx = _arrays(mesh)
    # 2 caps * 4 verts + 4 walls * 4 verts.
    assert pos.shape == (24, 3)
    assert norm.shape == (24, 3)
    assert uv.shape == (24, 2)
    # 2 caps * 2 tris + 4 walls * 2 tris, flat indices.
    assert idx.shape == (36,)
    assert pos.dtype == np.float32
    assert norm.dtype == np.float32
    assert uv.dtype == np.float32
    assert idx.dtype == np.uint32
    assert "vertices=24" in repr(mesh)


def test_square_top_face_area():
    pos, norm, _uv, idx = _arrays(build_prism_mesh(RECT))
    assert _top_face_area(pos, norm, idx) == pytest.approx(
        200.0 * 100.0, rel=1e-5
    )


def test_concave_polygon_area():
    poly = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 5.0),
        (5.0, 5.0),
        (5.0, 10.0),
        (0.0, 10.0),
    ]
    pos, norm, _uv, idx = _arrays(build_prism_mesh(poly))
    assert _top_face_area(pos, norm, idx) == pytest.approx(75.0, rel=1e-5)


def test_hole_counts_and_area():
    pos, norm, _uv, idx = _arrays(build_prism_mesh(RECT, [HOLE]))
    # 2 caps * 8 verts + 8 walls * 4 verts.
    assert pos.shape == (48, 3)
    # 2 caps * 8 tris (hole bridging adds two) + 8 walls * 2 tris.
    assert idx.shape == (96,)
    assert _top_face_area(pos, norm, idx) == pytest.approx(
        200.0 * 100.0 - 100.0 * 50.0, rel=1e-5
    )


def test_two_holes_area():
    hole1 = [(50.0, 25.0), (110.0, 25.0), (110.0, 55.0), (50.0, 55.0)]
    hole2 = [(130.0, 40.0), (190.0, 40.0), (190.0, 70.0), (130.0, 70.0)]
    pos, norm, _uv, idx = _arrays(build_prism_mesh(RECT, [hole1, hole2]))
    assert _top_face_area(pos, norm, idx) == pytest.approx(
        200.0 * 100.0 - 2 * 60.0 * 30.0, rel=1e-5
    )


def test_z_span_defaults():
    pos = np.asarray(build_prism_mesh(RECT).positions)
    assert pos[:, 2].max() == 0.0
    assert pos[:, 2].min() == -18.0


def test_custom_z_top_and_thickness():
    pos = np.asarray(
        build_prism_mesh(RECT, thickness=2.0, z_top=5.0).positions
    )
    assert pos[:, 2].max() == 5.0
    assert pos[:, 2].min() == 3.0


def test_unit_normals():
    norm = np.asarray(build_prism_mesh(RECT, [HOLE]).normals)
    assert np.allclose(np.linalg.norm(norm, axis=1), 1.0, atol=1e-6)


def test_wall_normals_horizontal():
    pos, norm, _uv, _idx = _arrays(build_prism_mesh(RECT, [HOLE]))
    walls = np.abs(norm[:, 2]) < 0.5
    assert np.allclose(norm[walls, 2], 0.0)


def test_wall_normals_face_away_from_solid():
    pos, norm, _uv, _idx = _arrays(build_prism_mesh(RECT, [HOLE]))
    walls = np.abs(norm[:, 2]) < 0.5
    outer_centroid = np.array([100.0, 50.0])
    hole_centroid = np.array([100.0, 50.0])
    for i in np.flatnonzero(walls):
        x, y = pos[i, 0], pos[i, 1]
        on_hole_ring = 40.0 < x < 160.0 and 15.0 < y < 85.0
        outward = (
            hole_centroid - pos[i, :2]
            if on_hole_ring
            else pos[i, :2] - outer_centroid
        )
        assert np.dot(norm[i, :2], outward) > 0.0


def test_uv_physical_density():
    _pos, _norm, uv, _idx = _arrays(build_prism_mesh(RECT, uv_scale=50.0))
    # 200x100 mm at 50 mm/tile => UV range (4, 2).
    assert np.isclose(uv[:, 0].max(), 4.0)
    assert np.isclose(uv[:, 1].max(), 2.0)
    assert np.isclose(uv.min(), 0.0)


def test_input_winding_normalized():
    ccw = _arrays(build_prism_mesh(RECT, [HOLE]))
    cw = _arrays(
        build_prism_mesh(list(reversed(RECT)), [list(reversed(HOLE))])
    )
    for a, b in zip(ccw, cw):
        assert a.shape == b.shape
    assert _top_face_area(cw[0], cw[1], cw[3]) == pytest.approx(
        _top_face_area(ccw[0], ccw[1], ccw[3]), rel=1e-5
    )


def test_default_uv_scale():
    _pos, _norm, uv, _idx = _arrays(build_prism_mesh(RECT))
    assert np.isclose(uv[:, 0].max(), 200.0 / 300.0)


@pytest.mark.parametrize(
    "kwargs",
    [
        {"outer": RECT[:2]},
        {"outer": RECT, "holes": [[(0.0, 0.0), (1.0, 1.0)]]},
        {"outer": RECT, "thickness": 0.0},
        {"outer": RECT, "thickness": -1.0},
        {"outer": RECT, "uv_scale": 0.0},
        {"outer": RECT, "uv_scale": -5.0},
    ],
)
def test_invalid_arguments_raise(kwargs):
    with pytest.raises(ValueError):
        build_prism_mesh(**kwargs)

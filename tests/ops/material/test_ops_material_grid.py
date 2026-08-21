"""Tests for raygeo.ops.material.grid.compute_power_uvs."""

import numpy as np
import pytest

from raygeo.mesh.build import build_prism_mesh
from raygeo.ops.material.grid import compute_power_uvs

RECT = [(0.0, 0.0), (200.0, 0.0), (200.0, 100.0), (0.0, 100.0)]


def _mesh_positions():
    return np.asarray(build_prism_mesh(RECT).positions, dtype=np.float32)


def test_power_uvs_map_grid_corners():
    """Vertices at the grid corners map to (0, 0) and (1, 1); every
    value stays within [0, 1] when the grid covers the mesh."""
    pos = _mesh_positions()
    puv = np.asarray(
        compute_power_uvs(pos, (0.0, 0.0), (50.0, 50.0), (10000, 5000)),
        dtype=np.float32,
    )
    assert puv.shape == pos.shape[:1] + (2,)
    assert puv.min() >= 0.0
    assert puv.max() <= 1.0
    at_origin = puv[(pos[:, 0] == 0.0) & (pos[:, 1] == 0.0)]
    assert at_origin.shape[0] > 0
    assert np.allclose(at_origin, 0.0)
    at_far = puv[(pos[:, 0] == 200.0) & (pos[:, 1] == 100.0)]
    assert np.allclose(at_far, 1.0)


def test_power_uvs_offset_grid():
    """A grid placed away from the origin shifts the mapping."""
    pos = _mesh_positions()
    puv = np.asarray(
        compute_power_uvs(pos, (100.0, -50.0), (10.0, 10.0), (1000, 1000)),
        dtype=np.float32,
    )
    at_origin = puv[(pos[:, 0] == 0.0) & (pos[:, 1] == 0.0)]
    u, v = at_origin[0]
    # (0, 0) mm is 100 mm left of / 50 mm above the grid origin.
    assert u == pytest.approx((-100.0 * 10.0) / 1000.0)
    assert v == pytest.approx((50.0 * 10.0) / 1000.0)


def test_power_uvs_index_aligned_with_positions():
    """Each power-uv row corresponds to the same vertex row."""
    pos = _mesh_positions()
    puv = np.asarray(
        compute_power_uvs(pos, (0.0, 0.0), (1.0, 1.0), (200, 100)),
        dtype=np.float32,
    )
    assert len(puv) == len(pos)
    for i, (x, y, _z) in enumerate(pos):
        assert puv[i, 0] == pytest.approx(x / 200.0)
        assert puv[i, 1] == pytest.approx(y / 100.0)


def test_power_uvs_rejects_non_3_columns():
    with pytest.raises(ValueError):
        compute_power_uvs(
            np.zeros((4, 2), dtype=np.float32),
            (0.0, 0.0),
            (10.0, 10.0),
            (100, 100),
        )

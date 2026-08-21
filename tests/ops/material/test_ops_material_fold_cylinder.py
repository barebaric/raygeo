"""Cylinder-stock folding tests for raygeo.ops.material."""

import numpy as np
import pytest

from raygeo.geo import Matrix
from raygeo.ops.material import RasterEffect, VectorEffect
from raygeo.ops.material.fold import fold_effects
from raygeo.ops.material.spec import (
    CylinderStock,
    FoldEntry,
    MaterialFoldSpec,
)

DIAMETER = 20.0
LENGTH = 40.0


def _fold(entries, grid=None, diameter=DIAMETER, length=LENGTH):
    spec = MaterialFoldSpec(
        stock=CylinderStock(diameter=diameter, length=length),
        entries=entries,
        grid=grid,
    )
    return fold_effects(spec)


def _raster(values, origin_mm=(0.0, 0.0), px_per_mm=(1.0, 1.0)):
    return RasterEffect(
        np.asarray(values, dtype=np.uint8),
        origin_mm=origin_mm,
        px_per_mm=px_per_mm,
    )


def test_cylindrical_profile_tag():
    state = _fold([])
    assert state.profile == "cylindrical"


def test_invalid_dimensions_rejected():
    with pytest.raises(ValueError):
        _fold([], diameter=0.0)
    with pytest.raises(ValueError):
        _fold([], length=-1.0)


def test_raster_lands_on_unrolled_domain():
    """A raster at axial 5..15 mm / arc 7..9 mm appears on the grid at
    the same world coordinates. The arc axis is centered on the
    machine origin: y spans [-pi*d/2, pi*d/2]."""
    values = np.zeros((10, 10), dtype=np.uint8)
    values[7:9, 5:15] = 180
    state = _fold(
        [FoldEntry("w1", Matrix.identity(), [_raster(values)])],
    )
    assert state.surface_map is not None
    assert state.grid is not None
    half_circ = np.pi * DIAMETER / 2.0
    assert state.grid.origin_mm == pytest.approx((0.0, -half_circ))
    sm = state.surface_map.to_numpy()
    ppm = state.grid.px_per_mm[0]
    col = int(10 * ppm) // 2
    row = int((8.0 + half_circ) * ppm)
    assert sm[row, col] == 180
    assert sm[0, col] == 0


def test_grid_covers_full_unrolled_domain():
    """The grid spans the whole circumference regardless of where the
    rasters sit, so the shell can sample anywhere."""
    values = np.full((2, 2), 255, dtype=np.uint8)
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [_raster(values, origin_mm=(1.0, 1.0))],
            )
        ],
    )
    assert state.grid is not None
    width_mm, height_mm = state.grid.size_px[0], state.grid.size_px[1]
    ppm_x, ppm_y = state.grid.px_per_mm
    assert width_mm == pytest.approx(LENGTH * ppm_x, rel=0.01)
    assert height_mm == pytest.approx(np.pi * DIAMETER * ppm_y, rel=0.01)
    assert state.grid.origin_mm == pytest.approx(
        (0.0, -np.pi * DIAMETER / 2.0)
    )


def test_vector_effects_are_ignored_without_provenance():
    def _rect(x, y, w, h):
        return [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]

    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [VectorEffect([_rect(1, 1, 2, 2)])],
            )
        ]
    )
    assert state.surface_map is None
    assert state.provenance == []
    assert state.void_polygons == []


def test_no_rasters_means_no_surface_map():
    state = _fold([])
    assert state.surface_map is None
    assert state.grid is None

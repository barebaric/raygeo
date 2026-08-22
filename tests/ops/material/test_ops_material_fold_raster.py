"""Raster-effect folding tests for raygeo.ops.material."""

import numpy as np
import pytest

from raygeo.geo import Matrix
from raygeo.ops.material import RasterEffect, VectorEffect
from raygeo.ops.material.fold import fold_effects
from raygeo.ops.material.spec import (
    FoldEntry,
    GridBudget,
    MaterialFoldSpec,
    PrismaticStock,
)

STOCK = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]


def _fold(entries, grid=None):
    spec = MaterialFoldSpec(
        stock=PrismaticStock(polygons=STOCK, thickness=3.0),
        entries=entries,
        grid=grid,
    )
    return fold_effects(spec)


def _raster(values, origin_mm=(0.0, 0.0), px_per_mm=(1.0, 1.0)):
    return RasterEffect(
        np.asarray(values, dtype=np.float32),
        origin_mm=origin_mm,
        px_per_mm=px_per_mm,
    )


def test_no_raster_entries_means_no_surface_map():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [VectorEffect([[(1, 1), (2, 1), (2, 2), (1, 2)]])],
            )
        ]
    )
    assert state.surface_map is None
    assert state.grid is None


def test_single_raster_lands_at_world_coordinates():
    line = np.zeros((10, 10), dtype=np.uint8)
    line[4, :] = 200
    state = _fold([FoldEntry("w1", Matrix.identity(), [_raster(line)])])
    assert state.surface_map is not None
    sm = state.surface_map.to_numpy()
    assert state.grid is not None
    assert state.grid.origin_mm == (0.0, 0.0)
    row = state.grid.size_px[1] * 4 // 10
    assert sm[row, :].max() == 200
    assert sm[row - 1, :].max() == 0


def test_raster_origin_offsets_world_position():
    line = np.zeros((10, 10), dtype=np.uint8)
    line[0, :] = 255
    state = _fold(
        [
            FoldEntry(
                "w1", Matrix.identity(), [_raster(line, origin_mm=(5.0, 5.0))]
            )
        ]
    )
    assert state.surface_map is not None
    sm = state.surface_map.to_numpy()
    assert sm.max() == 255
    assert state.grid is not None
    grid = state.grid
    v = int(5.0 * grid.px_per_mm[1])
    assert sm[v, :].max() == 255
    assert sm[max(v - 2, 0), :].max() == 0


def test_overlapping_rasters_max_reduce():
    a = np.zeros((10, 10), dtype=np.uint8)
    a[:, :] = 100
    b = np.zeros((10, 10), dtype=np.uint8)
    b[0:5, :] = 200
    state = _fold(
        [
            FoldEntry("w1", Matrix.identity(), [_raster(a)]),
            FoldEntry("w2", Matrix.identity(), [_raster(b)]),
        ]
    )
    assert state.surface_map is not None
    sm = state.surface_map.to_numpy()
    assert sm.min() == 100
    assert sm.max() == 200


def test_raster_placement_translates_sampling():
    square = np.zeros((10, 10), dtype=np.uint8)
    square[0:5, 0:5] = 255
    state = _fold(
        [FoldEntry("w1", Matrix.translation(5.0, 5.0), [_raster(square)])]
    )
    assert state.surface_map is not None
    sm = state.surface_map.to_numpy()
    assert state.grid is not None
    grid = state.grid
    v = int(5.0 * grid.px_per_mm[1])
    assert sm[v:, v:].max() == 255
    assert sm[:v, :].max() == 0


def test_grid_budget_caps_resolution():
    state = _fold(
        [
            FoldEntry(
                "w1", Matrix.identity(), [_raster(np.zeros((4, 4), np.uint8))]
            )
        ],
        grid=GridBudget(px_per_mm=100.0, max_px=64),
    )
    assert state.grid is not None
    w, h = state.grid.size_px
    assert w <= 64 and h <= 64
    assert state.grid.px_per_mm[0] == pytest.approx(6.4)


def test_default_grid_budget():
    state = _fold(
        [
            FoldEntry(
                "w1", Matrix.identity(), [_raster(np.zeros((4, 4), np.uint8))]
            )
        ],
    )
    assert state.grid is not None
    w, h = state.grid.size_px
    assert w == 500 and h == 500


def test_empty_power_map_rejected():
    with pytest.raises(ValueError):
        _raster(np.zeros((0, 0), dtype=np.uint8))


def test_invalid_grid_budget_rejected():
    with pytest.raises(ValueError):
        _fold(
            [
                FoldEntry(
                    "w1",
                    Matrix.identity(),
                    [_raster(np.zeros((4, 4), np.uint8))],
                )
            ],
            grid=GridBudget(px_per_mm=0.0, max_px=64),
        )


def test_surface_map_reflects_stock_extent():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [_raster(np.full((10, 10), 50, dtype=np.uint8))],
            )
        ]
    )
    assert state.surface_map is not None
    sm = state.surface_map.to_numpy()
    assert sm.shape == (500, 500)
    assert sm.min() == 50


@pytest.mark.slow
def test_large_raster_folds_reasonably():
    big = np.full((1024, 1024), 255, dtype=np.uint8)
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [_raster(big, px_per_mm=(100.0, 100.0))],
            )
        ],
        grid=GridBudget(px_per_mm=100.0, max_px=2048),
    )
    assert state.surface_map is not None
    sm = state.surface_map.to_numpy()
    assert sm.shape == (1000, 1000)
    assert sm.max() == 255

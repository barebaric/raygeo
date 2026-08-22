"""Tests for the physical fluence burn model.

Exercises the Python-visible parts of the fluence pipeline:
`RasterEffect` carries F32 fluence, the fold max-reduces fluence into
the `surface_map` (F32), and `MaterialFoldSpec`/`MaterialState` carry
the laser's wavelength and optical power for the renderer's
absorption lookup.
"""

import numpy as np

from raygeo.geo import Matrix
from raygeo.ops.material import RasterEffect
from raygeo.ops.material.fold import fold_effects
from raygeo.ops.material.spec import (
    FoldEntry,
    MaterialFoldSpec,
    PrismaticStock,
)

STOCK = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]


def _fold(entries, wavelength_nm=0.0, max_power_watts=0.0, grid=None):
    spec = MaterialFoldSpec(
        stock=PrismaticStock(polygons=STOCK, thickness=3.0),
        entries=entries,
        grid=grid,
        wavelength_nm=wavelength_nm,
        max_power_watts=max_power_watts,
    )
    return fold_effects(spec)


def _raster_fluence(values, origin_mm=(0.0, 0.0), px_per_mm=(1.0, 1.0)):
    return RasterEffect(
        np.asarray(values, dtype=np.float32),
        origin_mm=origin_mm,
        px_per_mm=px_per_mm,
    )


class TestRasterEffectFluence:
    def test_fluence_carried_as_f32(self):
        fx = _raster_fluence(np.full((4, 4), 7.5, dtype=np.float32))
        arr = fx.fluence.to_numpy()
        assert arr.dtype == np.float32
        assert arr.shape == (4, 4)
        assert arr[0, 0] == 7.5

    def test_empty_fluence_rejected(self):
        import pytest

        with pytest.raises(ValueError):
            _raster_fluence(np.zeros((0, 0), dtype=np.float32))

    def test_cut_fluence_threshold_optional(self):
        fx = RasterEffect(
            np.zeros((2, 2), dtype=np.float32),
            origin_mm=(0.0, 0.0),
            px_per_mm=(1.0, 1.0),
            cut_fluence_threshold=15.0,
        )
        assert fx.cut_fluence_threshold == 15.0


class TestFoldFluenceMaxReduce:
    def test_fluence_values_preserved(self):
        line = np.zeros((10, 10), dtype=np.float32)
        line[4, :] = 42.0
        state = _fold(
            [FoldEntry("w1", Matrix.identity(), [_raster_fluence(line)])]
        )
        assert state.surface_map is not None
        sm = state.surface_map.to_numpy()
        assert sm.dtype == np.float32
        assert state.grid is not None
        row = state.grid.size_px[1] * 4 // 10
        assert sm[row, :].max() == 42.0

    def test_overlapping_fluences_max_reduce(self):
        a = np.full((10, 10), 10.0, dtype=np.float32)
        b = np.zeros((10, 10), dtype=np.float32)
        b[0:5, :] = 30.0
        state = _fold(
            [
                FoldEntry("w1", Matrix.identity(), [_raster_fluence(a)]),
                FoldEntry("w2", Matrix.identity(), [_raster_fluence(b)]),
            ]
        )
        assert state.surface_map is not None
        sm = state.surface_map.to_numpy()
        assert sm.min() == 10.0
        assert sm.max() == 30.0

    def test_zero_fluence_raster_still_folds(self):
        z = np.zeros((4, 4), dtype=np.float32)
        state = _fold(
            [FoldEntry("w1", Matrix.identity(), [_raster_fluence(z)])]
        )
        # All-zero fluence still produces a surface map (the fold does
        # not skip zero rasters, only empty grids).
        assert state.surface_map is not None
        assert state.surface_map.to_numpy().max() == 0.0


class TestWavelengthPowerProvenance:
    def test_wavelength_carried_to_state(self):
        line = np.zeros((4, 4), dtype=np.float32)
        line[0, 0] = 5.0
        state = _fold(
            [FoldEntry("w1", Matrix.identity(), [_raster_fluence(line)])],
            wavelength_nm=10600.0,
            max_power_watts=60.0,
        )
        assert state.wavelength_nm == 10600.0
        assert state.max_power_watts == 60.0

    def test_defaults_to_zero_when_unconfigured(self):
        line = np.zeros((4, 4), dtype=np.float32)
        line[0, 0] = 5.0
        state = _fold(
            [FoldEntry("w1", Matrix.identity(), [_raster_fluence(line)])]
        )
        assert state.wavelength_nm == 0.0
        assert state.max_power_watts == 0.0

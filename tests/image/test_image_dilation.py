"""Tests for the ``radius_px`` dilation brush in ``rasterize_scanlines``.

Each rasterized sample is expanded to a square brush of side
``2*radius_px + 1`` (max-merged), equivalent to a square morphological
dilation of the thin raster. Coverage is bounds-clamped (never wrapped).
"""

import numpy as np

from raygeo.image import rasterize_scanlines
from raygeo.ops import Ops

# Test grid: 1 mm == 10 px, 100x100 texture, origin at (0, 0).
PPM = 10.0
W = 100
H = 100


def _raster(ops, w, h, ppm, **kw):
    """Call rasterize_scanlines and decompress the result."""
    return rasterize_scanlines(ops, w, h, ppm, **kw).to_numpy()


def _horizontal_ops(y_mm=5.0, x0_mm=0.0, x1_mm=10.0, power=255):
    """A single horizontal scanline at *y_mm* spanning [x0, x1] mm."""
    ops = Ops()
    ops.move_to(x0_mm, y_mm, 0.0)
    n_steps = int(round((x1_mm - x0_mm) * PPM))
    ops.scan_to(
        x1_mm,
        y_mm,
        0.0,
        power_values=bytes([power] * n_steps),
    )
    return ops


def _row_of(y_mm):
    """Pixel row index for a given mm y (texture origin is bottom-left)."""
    return int(round(H - y_mm * PPM))


def _filled_rows(buf):
    return np.where(buf.any(axis=1))[0]


def _filled_cols(buf):
    return np.where(buf.any(axis=0))[0]


def test_default_radius_is_zero():
    """Omitting radius_px must match an explicit radius_px=0."""
    ops = _horizontal_ops()
    default = _raster(ops, W, H, (PPM, PPM))
    explicit = _raster(ops, W, H, (PPM, PPM), radius_px=0)
    np.testing.assert_array_equal(default, explicit)


def test_radius_zero_is_single_pixel_thick():
    buf = _raster(_horizontal_ops(), W, H, (PPM, PPM), radius_px=0)
    rows = _filled_rows(buf)
    assert rows.size == 1
    assert rows[0] == _row_of(5.0)


def test_horizontal_thickness_matches_radius():
    """A horizontal scanline dilated by *r* spans exactly 2r+1 rows."""
    iy = _row_of(5.0)
    for r in (2, 5, 7, 12):
        buf = _raster(_horizontal_ops(), W, H, (PPM, PPM), radius_px=r)
        rows = _filled_rows(buf)
        thickness = rows.max() - rows.min() + 1
        assert thickness == 2 * r + 1, f"radius {r}: got {thickness}"
        assert rows.min() == iy - r
        assert rows.max() == iy + r


def test_horizontal_width_grows_with_radius():
    """A finite scanline's column extent also expands by the brush."""
    ops = _horizontal_ops(x0_mm=2.0, x1_mm=8.0)
    thin = _raster(ops, W, H, (PPM, PPM), radius_px=0)
    thick = _raster(ops, W, H, (PPM, PPM), radius_px=4)
    thin_w = _filled_cols(thin).max() - _filled_cols(thin).min() + 1
    thick_w = _filled_cols(thick).max() - _filled_cols(thick).min() + 1
    assert thick_w - thin_w == 2 * 4


def test_thicker_radius_gives_more_filled_pixels():
    ops = _horizontal_ops()
    thin = _raster(ops, W, H, (PPM, PPM), radius_px=2)
    thick = _raster(ops, W, H, (PPM, PPM), radius_px=7)
    assert int(thick.sum()) > int(thin.sum())


def test_no_wraparound_at_top_edge():
    """A scanline on the top row must not bleed onto the bottom rows.

    Regression for the old ``np.roll`` wraparound bug, where content near
    one texture edge reappeared on the opposite edge.
    """
    ops = _horizontal_ops(y_mm=10.0)  # iy == 0 (top row)
    buf = _raster(ops, W, H, (PPM, PPM), radius_px=5)
    rows = _filled_rows(buf)
    assert rows.min() == 0
    assert rows.max() == 5
    # Bottom half must be entirely empty (no wraparound).
    assert not buf[H // 2 :].any()


def test_no_wraparound_at_left_edge():
    """A scanline starting at x=0 must not bleed onto the right columns."""
    ops = _horizontal_ops(y_mm=5.0, x0_mm=0.0, x1_mm=1.0)
    buf = _raster(ops, W, H, (PPM, PPM), radius_px=5)
    cols = _filled_cols(buf)
    assert cols.min() == 0
    # Pixel-center coverage: the swept interval [0, 10] px dilated by the
    # radius reaches x = 15, and the last column whose center (k + 0.5)
    # lies inside is k = 14.
    assert cols.max() == 10 + 5 - 1
    # Right half must be entirely empty.
    assert not buf[:, W // 2 :].any()


def test_power_max_merge():
    """A lower power drawn after a higher one must not overwrite it."""
    ops = Ops()
    ops.move_to(0.0, 5.0, 0.0)
    ops.scan_to(1.0, 5.0, 0.0, power_values=bytes([200]))
    ops.move_to(0.0, 5.0, 0.0)
    ops.scan_to(1.0, 5.0, 0.0, power_values=bytes([100]))

    buf = _raster(ops, W, H, (PPM, PPM), radius_px=3)
    nonzero = buf[buf > 0]
    assert nonzero.size > 0
    assert nonzero.min() == 200


def test_diagonal_scanline_dilates():
    """A non-horizontal scanline is thickened along its whole length."""
    ops = Ops()
    ops.move_to(2.0, 2.0, 0.0)
    ops.scan_to(8.0, 8.0, 0.0, power_values=bytes([255] * 60))
    thin = _raster(ops, W, H, (PPM, PPM), radius_px=0)
    thick = _raster(ops, W, H, (PPM, PPM), radius_px=4)
    assert int(thick.sum()) > int(thin.sum())

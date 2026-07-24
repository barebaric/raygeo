"""
Tests for :mod:`raygeo.ops.convert.view`.

These exercise the pure Rust renderer (``src/ops/convert/view.rs``) via
the PyO3 bindings to:

- confirm empty Ops with degenerate bbox returns ``None``,
- verify bounding-box, dimensions and pixel content for a single
  powered cut line,
- check the cut LUT is consulted and power-modulated alpha is applied,
- confirm travel / zero-power colour routing and the
  ``show_travel_moves`` toggle,
- exercise the multi-workpiece batch entry point,
- exercise the ``ViewSpec`` encoder interface.
"""

import numpy as np
import pytest

from raygeo.ops import Ops
from raygeo.ops.convert import ViewSpec
from raygeo.ops.convert.view import render_ops, render_ops_batch


def _default_lut() -> list:
    """256-entry cut LUT — opaque black across the board."""
    return [[0, 0, 0, 255] for _ in range(256)]


def _default_engrave_lut() -> list:
    """256-entry engrave LUT — transparent by default."""
    return [[0, 0, 0, 0] for _ in range(256)]


def _render(
    ops,
    bbox,
    *,
    pixels_per_mm=(8.0, 8.0),
    show_travel_moves=True,
    cut_color=None,
    travel_color=None,
    zero_power_color=None,
    cut_lut=None,
    engrave_lut=None,
    **kwargs,
):
    """Convenience: render with default black-on-transparent colours."""
    spec = ViewSpec(
        pixels_per_mm=pixels_per_mm,
        render_bbox=bbox,
        cut_color=cut_color or [0, 0, 0, 255],
        travel_color=travel_color or [0, 255, 255, 255],
        zero_power_color=zero_power_color or [128, 128, 128, 255],
        cut_lut=cut_lut or _default_lut(),
        engrave_lut=engrave_lut or _default_engrave_lut(),
        show_travel_moves=show_travel_moves,
        **kwargs,
    )
    return render_ops(ops, spec)


# ---------------------------------------------------------------------------
# Empty / trivial inputs
# ---------------------------------------------------------------------------


def test_empty_ops_with_degenerate_bbox_returns_none():
    ops = Ops()
    result = _render(ops, bbox=(0.0, 0.0, 0.0, 0.0))
    assert result is None


def test_only_travel_moves_with_show_travel_disabled():
    """Even with a valid bbox, travel-only ops still renders (the
    texture path may produce nothing, but the bitmap is allocated).
    However, the bitmap should be all zeros since no powered segments
    and no texture."""
    ops = Ops()
    ops.move_to(0.0, 0.0)
    ops.move_to(10.0, 10.0)
    result = _render(ops, bbox=(0.0, 0.0, 10.0, 10.0), show_travel_moves=False)
    assert result is not None
    assert result.bitmap[:, :, 3].max() == 0


# ---------------------------------------------------------------------------
# Single powered cut line
# ---------------------------------------------------------------------------


def _cut_line_ops() -> Ops:
    """Move to (0,0), set power 1.0, line to (10,10)."""
    ops = Ops()
    ops.move_to(0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(10.0, 10.0)
    return ops


def test_cut_line_produces_bitmap_with_expected_dimensions():
    ops = _cut_line_ops()
    result = _render(
        ops, bbox=(0.0, 0.0, 10.0, 10.0), pixels_per_mm=(8.0, 8.0)
    )
    assert result is not None

    min_x, min_y, max_x, max_y = result.bbox_mm
    assert min_x == pytest.approx(0.0, abs=1e-6)
    assert min_y == pytest.approx(0.0, abs=1e-6)
    assert max_x == pytest.approx(10.0, abs=1e-6)
    assert max_y == pytest.approx(10.0, abs=1e-6)

    assert result.effective_ppm[0] == pytest.approx(8.0, abs=1e-6)
    assert result.effective_ppm[1] == pytest.approx(8.0, abs=1e-6)

    assert result.bitmap.shape == (80, 80, 4)
    assert result.bitmap.dtype == np.uint8


def test_cut_line_writes_nonzero_pixels():
    ops = _cut_line_ops()
    result = _render(
        ops, bbox=(0.0, 0.0, 10.0, 10.0), pixels_per_mm=(8.0, 8.0)
    )
    assert result is not None

    alpha = result.bitmap[:, :, 3]
    assert alpha.max() > 0
    rgb_sum = result.bitmap[:, :, :3].sum(axis=2)
    stroke_mask = (alpha > 0) & (rgb_sum == 0)
    assert stroke_mask.any()


# ---------------------------------------------------------------------------
# LUT / power-modulated alpha
# ---------------------------------------------------------------------------


def _powered_line_ops(power: float) -> Ops:
    ops = Ops()
    ops.move_to(0.0, 0.0)
    ops.set_power(power)
    ops.line_to(10.0, 10.0)
    return ops


def test_lut_entry_for_full_power_used_for_color():
    ops = _powered_line_ops(1.0)
    lut = [[0, 0, 0, 255] for _ in range(256)]
    lut[255] = [0, 0, 255, 255]
    result = _render(
        ops,
        bbox=(0.0, 0.0, 10.0, 10.0),
        pixels_per_mm=(8.0, 8.0),
        cut_color=[0, 0, 0, 255],
        cut_lut=lut,
    )
    assert result is not None
    # ARGB32 on LE: channel 2 = Red
    r = result.bitmap[:, :, 2]
    assert r.max() > 0


def test_lut_alpha_remapped_to_minimum_half():
    """The alpha remap ``alpha' = alpha * 0.5 + 0.5`` gives even
    low-alpha LUT entries a minimum opacity of ~128."""
    ops = _powered_line_ops(1.0)
    lut = [[0, 0, 0, 10] for _ in range(256)]
    result = _render(
        ops,
        bbox=(0.0, 0.0, 10.0, 10.0),
        pixels_per_mm=(8.0, 8.0),
        cut_color=[0, 0, 0, 255],
        cut_lut=lut,
    )
    assert result is not None
    alpha = result.bitmap[:, :, 3]
    assert alpha.max() > 100  # remapped: 10*0.5+127.5 ≈ 132


def test_lut_must_have_exactly_256_entries():
    ops = _cut_line_ops()
    with pytest.raises(ValueError, match="256"):
        render_ops(
            ops,
            ViewSpec(
                pixels_per_mm=(1.0, 1.0),
                render_bbox=(0.0, 0.0, 10.0, 10.0),
                cut_color=[0, 0, 0, 255],
                travel_color=[255, 255, 0, 255],
                zero_power_color=[128, 128, 128, 255],
                cut_lut=[[0, 0, 0, 255]] * 10,
                engrave_lut=_default_engrave_lut(),
            ),
        )


def test_engrave_lut_must_have_exactly_256_entries():
    ops = _cut_line_ops()
    with pytest.raises(ValueError, match="256"):
        render_ops(
            ops,
            ViewSpec(
                pixels_per_mm=(1.0, 1.0),
                render_bbox=(0.0, 0.0, 10.0, 10.0),
                cut_color=[0, 0, 0, 255],
                travel_color=[255, 255, 0, 255],
                zero_power_color=[128, 128, 128, 255],
                cut_lut=_default_lut(),
                engrave_lut=[[0, 0, 0, 0]] * 10,
            ),
        )


# ---------------------------------------------------------------------------
# Travel / zero-power routing
# ---------------------------------------------------------------------------


def _ops_with_travel_and_zero_power() -> Ops:
    ops = Ops()
    ops.move_to(0.0, 0.0)
    ops.line_to(2.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(4.0, 0.0)
    ops.move_to(10.0, 0.0)
    return ops


def test_show_travel_moves_toggle_changes_pixel_count():
    ops = _ops_with_travel_and_zero_power()
    show = _render(
        ops,
        bbox=(0.0, -1.0, 10.0, 1.0),
        pixels_per_mm=(8.0, 8.0),
        show_travel_moves=True,
    )
    hide = _render(
        ops,
        bbox=(0.0, -1.0, 10.0, 1.0),
        pixels_per_mm=(8.0, 8.0),
        show_travel_moves=False,
    )
    assert show is not None and hide is not None

    show_alpha = int(show.bitmap[:, :, 3].sum())
    hide_alpha = int(hide.bitmap[:, :, 3].sum())
    assert show_alpha >= hide_alpha
    assert show_alpha > hide_alpha


def test_zero_power_segments_use_zero_power_color():
    ops = _ops_with_travel_and_zero_power()
    result = _render(
        ops,
        bbox=(0.0, -1.0, 10.0, 1.0),
        pixels_per_mm=(8.0, 8.0),
        zero_power_color=[0, 0, 255, 255],
    )
    assert result is not None
    b = result.bitmap[:, :, 2]
    assert b.max() > 0


# ---------------------------------------------------------------------------
# Batch renderer
# ---------------------------------------------------------------------------


def test_render_ops_batch_returns_aligned_list():
    ops1 = _cut_line_ops()
    ops2 = Ops()

    ops3 = Ops()
    ops3.move_to(0.0, 0.0)
    ops3.set_power(1.0)
    ops3.line_to(5.0, 5.0)

    lut = _default_lut()
    spec = ViewSpec(
        pixels_per_mm=(4.0, 4.0),
        render_bbox=(0.0, 0.0, 10.0, 10.0),
        cut_color=[0, 0, 0, 255],
        travel_color=[255, 255, 0, 255],
        zero_power_color=[128, 128, 128, 255],
        cut_lut=lut,
        engrave_lut=_default_engrave_lut(),
    )
    items = [
        (ops1, spec),
        (ops2, spec),
        (ops3, spec),
    ]
    results = render_ops_batch(items)
    assert len(results) == 3
    assert results[0] is not None
    assert results[1] is not None  # empty ops, valid bbox → empty bitmap
    assert results[2] is not None
    # The empty-ops bitmap should be all zeros.
    assert results[1].bitmap[:, :, 3].max() == 0
    for r in [results[0], results[2]]:
        assert r.bitmap.ndim == 3
        assert r.bitmap.shape[2] == 4
        assert r.bitmap.dtype == np.uint8


def test_render_ops_batch_empty_input():
    assert render_ops_batch([]) == []


# ---------------------------------------------------------------------------
# Bbox / dimension clamping
# ---------------------------------------------------------------------------


def _big_cut_line_ops() -> Ops:
    ops = Ops()
    ops.move_to(0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(50.0, 50.0)
    return ops


def test_max_dimension_clamps_width():
    ops = _big_cut_line_ops()
    spec = ViewSpec(
        pixels_per_mm=(8.0, 8.0),
        render_bbox=(0.0, 0.0, 50.0, 50.0),
        cut_color=[0, 0, 0, 255],
        travel_color=[255, 255, 0, 255],
        zero_power_color=[128, 128, 128, 255],
        cut_lut=_default_lut(),
        engrave_lut=_default_engrave_lut(),
        max_dimension_px=100,
        max_total_pixels=8192 * 8192,
    )
    result = render_ops(ops, spec)
    assert result is not None
    assert result.bitmap.shape[0] <= 100
    assert result.bitmap.shape[1] <= 100
    assert result.effective_ppm[0] < 8.0


# ---------------------------------------------------------------------------
# ViewSpec construction
# ---------------------------------------------------------------------------


def test_viewspec_construction():
    spec = ViewSpec(
        pixels_per_mm=(4.0, 4.0),
        render_bbox=(0.0, 0.0, 10.0, 10.0),
        cut_color=[0, 0, 0, 255],
        travel_color=[255, 255, 0, 255],
        zero_power_color=[128, 128, 128, 255],
        cut_lut=_default_lut(),
        engrave_lut=_default_engrave_lut(),
    )
    assert spec.pixels_per_mm == (4.0, 4.0)
    assert spec.render_bbox == (0.0, 0.0, 10.0, 10.0)
    assert list(spec.cut_color) == [0, 0, 0, 255]


def test_viewspec_rejects_bad_lut_size():
    with pytest.raises(ValueError, match="256"):
        ViewSpec(
            pixels_per_mm=(4.0, 4.0),
            render_bbox=(0.0, 0.0, 10.0, 10.0),
            cut_color=[0, 0, 0, 255],
            travel_color=[255, 255, 0, 255],
            zero_power_color=[128, 128, 128, 255],
            cut_lut=[[0, 0, 0, 255]] * 10,
            engrave_lut=_default_engrave_lut(),
        )


def test_viewspec_defaults():
    spec = ViewSpec(
        pixels_per_mm=(1.0, 1.0),
        render_bbox=(0.0, 0.0, 10.0, 10.0),
        cut_color=[0, 0, 0, 255],
        travel_color=[0, 255, 0, 255],
        zero_power_color=[0, 0, 255, 255],
        cut_lut=_default_lut(),
        engrave_lut=_default_engrave_lut(),
    )
    assert spec.show_travel_moves is True
    assert spec.max_dimension_px == 8192
    assert spec.max_total_pixels == 8192 * 8192


def test_viewspec_repr():
    spec = ViewSpec(
        pixels_per_mm=(4.0, 4.0),
        render_bbox=(0.0, 0.0, 10.0, 10.0),
        cut_color=[255, 0, 0, 255],
        travel_color=[0, 255, 0, 255],
        zero_power_color=[0, 0, 255, 255],
        cut_lut=_default_lut(),
        engrave_lut=_default_engrave_lut(),
    )
    s = repr(spec)
    assert "ViewSpec" in s


# ---------------------------------------------------------------------------
# render_ops_into (chunk rendering)
# ---------------------------------------------------------------------------


def test_render_ops_into_writes_into_buffer():
    from raygeo.ops.convert.view import render_ops_into

    ops = _cut_line_ops()
    bitmap = np.zeros((80, 80, 4), dtype=np.uint8)
    spec = ViewSpec(
        pixels_per_mm=(8.0, 8.0),
        render_bbox=(0.0, 0.0, 10.0, 10.0),
        cut_color=[0, 0, 0, 255],
        travel_color=[255, 255, 0, 255],
        zero_power_color=[128, 128, 128, 255],
        cut_lut=_default_lut(),
        engrave_lut=_default_engrave_lut(),
    )
    result = render_ops_into(ops, spec, bitmap, (0.0, 0.0, 10.0, 10.0))
    assert result is True
    assert bitmap[:, :, 3].max() > 0


def test_render_ops_into_degenerate_bbox_returns_false():
    from raygeo.ops.convert.view import render_ops_into

    ops = _cut_line_ops()
    bitmap = np.zeros((80, 80, 4), dtype=np.uint8)
    spec = ViewSpec(
        pixels_per_mm=(8.0, 8.0),
        render_bbox=(0.0, 0.0, 10.0, 10.0),
        cut_color=[0, 0, 0, 255],
        travel_color=[255, 255, 0, 255],
        zero_power_color=[128, 128, 128, 255],
        cut_lut=_default_lut(),
        engrave_lut=_default_engrave_lut(),
    )
    result = render_ops_into(ops, spec, bitmap, (0.0, 0.0, 0.0, 0.0))
    assert result is False

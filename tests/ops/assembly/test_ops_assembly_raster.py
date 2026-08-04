"""Tests for the raster assembly module."""

import numpy as np
import pytest

from raygeo.ops.assembly.raster import raster
from raygeo.ops.part import Part
from raygeo.ops.types import CommandType, SectionType

# step_power is quantized to a u8 mask byte (0-255) and back, so
# comparisons need slack for the resulting rounding error.
_BYTE_TOL = 1 / 255


def _part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0), fill=255):
    part = Part(size_mm=size_mm, pixels_per_mm=pixels_per_mm)
    w = int(size_mm[0] * pixels_per_mm[0])
    h = int(size_mm[1] * pixels_per_mm[1])
    part.image = np.full((h, w), fill, dtype=np.uint8)
    return part


def _linearized_powers(ops):
    sl = ops.indices_of(CommandType.SCAN_LINE)
    powers = set()
    for i in sl:
        sub = ops.linearize(i, (0.0, 0.0, 0.0))
        powers.update(
            sub.power(j) for j in sub.indices_of(CommandType.SET_POWER)
        )
    return powers


def test_mask_scan_uses_step_power_not_max_power():
    """mask_scan must expose pixels at step_power, regardless of
    max_power (which is a power_modulated-only control and left at its
    default of 1.0 here)."""
    result = raster(
        _part(), mode="mask_scan", line_interval_mm=1.0, step_power=0.2
    )
    (power,) = _linearized_powers(result.ops)
    assert power == pytest.approx(0.2, abs=_BYTE_TOL)


def test_dither_uses_step_power_not_max_power():
    """dither shares mask_scan's code path and must behave the same way."""
    result = raster(
        _part(), mode="dither", line_interval_mm=1.0, step_power=0.35
    )
    (power,) = _linearized_powers(result.ops)
    assert power == pytest.approx(0.35, abs=_BYTE_TOL)


def test_mask_scan_default_step_power_matches_raster_default():
    """Sanity check: the default step_power (0.1) is respected, so
    existing callers that don't pass step_power are unaffected."""
    result = raster(_part(), mode="mask_scan", line_interval_mm=1.0)
    (power,) = _linearized_powers(result.ops)
    assert power == pytest.approx(0.1, abs=_BYTE_TOL)


def test_dot_width_correction_reaches_raster_entry_point():
    """Must flow through raster() itself, not just Ops.from_mask_scan."""
    baseline = raster(
        _part(), mode="mask_scan", line_interval_mm=1.0, step_power=1.0
    )
    trimmed = raster(
        _part(),
        mode="mask_scan",
        line_interval_mm=1.0,
        step_power=1.0,
        dot_width_correction_mm=0.2,
    )

    assert baseline.ops.len() == trimmed.ops.len()
    for i in range(baseline.ops.len()):
        assert baseline.ops.command_type(i) == trimmed.ops.command_type(i)
        assert baseline.ops.endpoint(i) == trimmed.ops.endpoint(i)

    sl_indices = trimmed.ops.indices_of(CommandType.SCAN_LINE)
    assert sl_indices
    data = trimmed.ops.scanline_data(sl_indices[0])
    assert data[0] == 0
    assert data[-1] == 0
    assert any(v > 0 for v in data)


def _section_z_sequence(ops):
    """Return the Z value of each RasterFill section in order."""
    sections = ops.sections()
    zs = []
    for sec in sections:
        if sec.section_type != SectionType.RASTER_FILL:
            continue
        content = ops.section_content(sec)
        for i in range(content.len()):
            if content.command_type(i) in (
                CommandType.LINE_TO,
                CommandType.SCAN_LINE,
            ):
                zs.append(content.endpoint(i)[2])
                break
    return zs


def test_cross_hatch_multi_pass_interleaves_per_pass():
    """Cross-hatch multi_pass must interleave angles per pass:
    pass1-angle0, pass1-angle90, pass2-angle0, pass2-angle90, ...
    not all-angle0 then all-angle90."""
    gray = np.full((20, 20), 0, dtype=np.uint8)
    part = _part(size_mm=(2.0, 2.0), pixels_per_mm=(10.0, 10.0))
    part.image = gray

    result = raster(
        part,
        mode="multi_pass",
        line_interval_mm=0.2,
        num_depth_levels=3,
        z_step_down=0.5,
        cross_hatch=True,
    )
    zs = _section_z_sequence(result.ops)
    assert len(zs) == 6
    assert zs == [0.0, 0.0, -0.5, -0.5, -1.0, -1.0]


def test_non_cross_hatch_multi_pass_keeps_pass_order():
    """Without cross-hatch, passes are in order: pass1, pass2, pass3."""
    gray = np.full((20, 20), 0, dtype=np.uint8)
    part = _part(size_mm=(2.0, 2.0), pixels_per_mm=(10.0, 10.0))
    part.image = gray

    result = raster(
        part,
        mode="multi_pass",
        line_interval_mm=0.2,
        num_depth_levels=3,
        z_step_down=0.5,
    )
    zs = _section_z_sequence(result.ops)
    assert len(zs) == 3
    assert zs == [0.0, -0.5, -1.0]

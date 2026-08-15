"""Tests for the split-sum BRDF integration LUT."""

import numpy as np
import pytest

from raygeo.image.pbr import generate_brdf_lut


def test_shape_and_dtype():
    lut = generate_brdf_lut(size=32)
    assert lut.shape == (32, 32, 2)
    assert lut.dtype == np.float32


def test_deterministic():
    a = generate_brdf_lut(size=16, sample_count=64)
    b = generate_brdf_lut(size=16, sample_count=64)
    assert np.array_equal(a, b)


def test_all_finite_and_non_negative():
    lut = generate_brdf_lut(size=32)
    assert np.isfinite(lut).all()
    assert (lut >= 0.0).all()


def test_energy_conservation():
    # scale + bias <= 1 everywhere (the LUT never adds energy).
    lut = generate_brdf_lut(size=32).astype(np.float64)
    assert (lut[..., 0] + lut[..., 1]).max() <= 1.0 + 1e-3


def test_mirror_corner():
    # Low roughness, NdotV -> 1: F0 is preserved (scale ~ 1, bias ~ 0).
    lut = generate_brdf_lut(size=32).astype(np.float64)
    scale, bias = lut[0, -1]
    assert scale == pytest.approx(1.0, abs=0.02)
    assert bias == pytest.approx(0.0, abs=1e-3)


def test_grazing_increases_bias():
    # At high roughness, grazing angles (low NdotV) lose more of F0
    # into the additive bias term than head-on angles.
    lut = generate_brdf_lut(size=32).astype(np.float64)
    bias = lut[..., 1]
    assert bias[-1, 0] > bias[-1, -1]
    assert bias.max() > 0.05


def test_grazing_scale_increases_with_roughness():
    # At grazing angles the IBL k remapping suppresses the smooth
    # surface's F0 term entirely, while rough surfaces scatter enough
    # light back for a nonzero scale (matches the canonical LUT's
    # dark bottom-left corner).
    lut = generate_brdf_lut(size=32).astype(np.float64)
    scale = lut[..., 0]
    assert scale[-1, 0] > scale[0, 0]


def test_larger_sample_count_converges():
    coarse = generate_brdf_lut(size=16, sample_count=64).astype(np.float64)
    fine = generate_brdf_lut(size=16, sample_count=4096).astype(np.float64)
    assert np.abs(coarse - fine).max() < 0.05


def test_more_samples_reduce_energy_at_mirror():
    # The mirror corner overshoots slightly with few samples (clamped
    # by float32); more samples must not move it past 1.
    fine = generate_brdf_lut(size=8, sample_count=4096).astype(np.float64)
    assert fine[0, -1, 0] <= 1.0 + 1e-3


@pytest.mark.parametrize("kwargs", [{"size": 0}, {"sample_count": 0}])
def test_invalid_arguments_raise(kwargs):
    with pytest.raises(ValueError):
        generate_brdf_lut(**kwargs)

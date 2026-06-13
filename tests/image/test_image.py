import numpy as np
import pytest

from raygeo.image import (
    apply_bayer_dither,
    apply_floyd_steinberg_dither,
    apply_minimum_run_length,
    compute_auto_levels,
    linear_to_srgb,
    normalize_grayscale,
    srgb_to_linear,
)

# ---------------------------------------------------------------------------
# srgb_to_linear
# ---------------------------------------------------------------------------


class TestSrgbToLinear:
    def test_black(self):
        result = srgb_to_linear(np.array([0], dtype=np.uint8))
        assert result[0] < 1e-6

    def test_white(self):
        result = srgb_to_linear(np.array([255], dtype=np.uint8))
        assert abs(result[0] - 1.0) < 0.01

    def test_midgray(self):
        result = srgb_to_linear(np.array([128], dtype=np.uint8))
        assert 0.0 < result[0] < 1.0

    def test_2d_shape(self):
        arr = np.array([[0, 128], [255, 64]], dtype=np.uint8)
        result = srgb_to_linear(arr)
        assert result.shape == (2, 2)

    def test_monotonic(self):
        vals = np.arange(256, dtype=np.uint8)
        result = srgb_to_linear(vals)
        for i in range(1, 256):
            assert result[i] > result[i - 1]


# ---------------------------------------------------------------------------
# linear_to_srgb
# ---------------------------------------------------------------------------


class TestLinearToSrgb:
    def test_zero(self):
        result = linear_to_srgb(np.array([0.0], dtype=np.float32))
        assert result[0] == 0

    def test_one(self):
        result = linear_to_srgb(np.array([1.0], dtype=np.float32))
        assert result[0] == 255

    def test_roundtrip(self):
        original = np.arange(256, dtype=np.uint8)
        linear = srgb_to_linear(original)
        recovered = linear_to_srgb(linear)
        diff = np.abs(original.astype(int) - recovered.astype(int))
        assert np.all(diff <= 1)

    def test_dither(self):
        vals = np.full(100, 0.5, dtype=np.float32)
        result = linear_to_srgb(vals, dither=True)
        assert result.shape == (100,)
        assert result.dtype == np.uint8

    def test_clamp_negative(self):
        result = linear_to_srgb(np.array([-1.0], dtype=np.float32))
        assert result[0] == 0

    def test_clamp_above_one(self):
        result = linear_to_srgb(np.array([2.0], dtype=np.float32))
        assert result[0] == 255


# ---------------------------------------------------------------------------
# compute_auto_levels
# ---------------------------------------------------------------------------


class TestComputeAutoLevels:
    def test_empty(self):
        bp, wp = compute_auto_levels(np.array([], dtype=np.uint8))
        assert bp == 0
        assert wp == 255

    def test_uniform(self):
        img = np.full(100, 128, dtype=np.uint8)
        bp, wp = compute_auto_levels(img)
        assert bp < wp

    def test_gradient(self):
        img = np.arange(256, dtype=np.uint8)
        bp, wp = compute_auto_levels(img, clip_percent=5.0)
        assert bp < wp


# ---------------------------------------------------------------------------
# normalize_grayscale
# ---------------------------------------------------------------------------


class TestNormalizeGrayscale:
    def test_identity(self):
        img = np.array([0, 128, 255], dtype=np.uint8)
        result = normalize_grayscale(img)
        assert result[0] == 0
        assert result[2] == 255

    def test_stretch(self):
        img = np.array([50, 100, 150, 200], dtype=np.uint8)
        result = normalize_grayscale(img, black_point=50, white_point=200)
        assert result[0] == 0
        assert result[3] == 255

    def test_invalid_raises(self):
        with pytest.raises(Exception):
            normalize_grayscale(
                np.array([128], dtype=np.uint8),
                black_point=128,
                white_point=128,
            )


# ---------------------------------------------------------------------------
# apply_floyd_steinberg_dither
# ---------------------------------------------------------------------------


class TestFloydSteinberg:
    def test_all_white(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        result = apply_floyd_steinberg_dither(gray, invert=False)
        assert np.all(result == 0)

    def test_all_black(self):
        gray = np.full((10, 10), 0, dtype=np.uint8)
        result = apply_floyd_steinberg_dither(gray, invert=False)
        assert np.all(result == 1)

    def test_invert(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        result = apply_floyd_steinberg_dither(gray, invert=True)
        assert np.all(result == 1)

    def test_binary_output(self):
        gray = np.random.randint(0, 256, (20, 20), dtype=np.uint8)
        result = apply_floyd_steinberg_dither(gray, invert=False)
        assert set(np.unique(result)).issubset({0, 1})

    def test_midgray_coverage(self):
        gray = np.full((50, 50), 128, dtype=np.uint8)
        result = apply_floyd_steinberg_dither(gray, invert=False)
        ones = np.sum(result)
        assert ones > 0
        assert ones < 50 * 50


# ---------------------------------------------------------------------------
# apply_minimum_run_length
# ---------------------------------------------------------------------------


class TestMinimumRunLength:
    def test_noop(self):
        binary = np.array([[1, 1, 1, 0, 1, 1, 1]], dtype=np.uint8)
        result = apply_minimum_run_length(binary, min_run_length=1)
        np.testing.assert_array_equal(result, binary)

    def test_removes_short(self):
        binary = np.array([[1, 1, 0, 1, 0, 1, 1, 1]], dtype=np.uint8)
        result = apply_minimum_run_length(binary, min_run_length=3)
        assert result[0, 3] == 0
        assert np.array_equal(result[0, 5:8], [1, 1, 1])


# ---------------------------------------------------------------------------
# apply_bayer_dither
# ---------------------------------------------------------------------------


class TestBayerDither:
    def test_basic(self):
        gray = np.full((4, 4), 128, dtype=np.uint8)
        bayer = np.array([[0, 2], [3, 1]], dtype=np.float32)
        result = apply_bayer_dither(gray, bayer, invert=False)
        ones = np.sum(result)
        assert ones > 0
        assert ones < 16

    def test_all_black(self):
        gray = np.full((4, 4), 0, dtype=np.uint8)
        bayer = np.array([[0, 2], [3, 1]], dtype=np.float32)
        result = apply_bayer_dither(gray, bayer, invert=False)
        assert np.all(result == 1)

    def test_all_white(self):
        gray = np.full((4, 4), 255, dtype=np.uint8)
        bayer = np.array([[0, 2], [3, 1]], dtype=np.float32)
        result = apply_bayer_dither(gray, bayer, invert=False)
        assert np.all(result == 0)

    def test_binary_output(self):
        gray = np.random.randint(0, 256, (20, 20), dtype=np.uint8)
        bayer = np.array([[0, 2], [3, 1]], dtype=np.float32)
        result = apply_bayer_dither(gray, bayer, invert=False)
        assert set(np.unique(result)).issubset({0, 1})

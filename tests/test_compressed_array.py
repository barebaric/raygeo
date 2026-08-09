"""Tests for the CompressedArray PyO3 type."""

import numpy as np

from raygeo.compressed_array import CompressedArray

# ── float32 round-trip ──────────────────────────────────────────


def test_float32_roundtrip():
    data = np.array([1.0, 2.5, -3.0, 0.0, 100.0, -0.001], dtype=np.float32)
    ca = CompressedArray.from_float32(data)
    result = ca.to_numpy()
    np.testing.assert_array_equal(result, data)
    assert result.dtype == np.float32


def test_float32_large_compression():
    rng = np.random.default_rng(42)
    data = rng.uniform(-100, 100, 100_000).astype(np.float32)
    ca = CompressedArray.from_float32(data)
    assert ca.ratio < 1.0
    np.testing.assert_allclose(ca.to_numpy(), data)


def test_float32_small_no_compression():
    data = np.array([1.0, 2.0, 3.0], dtype=np.float32)
    ca = CompressedArray.from_float32(data)
    assert ca.ratio == 1.0
    np.testing.assert_array_equal(ca.to_numpy(), data)


# ── int32 round-trip ────────────────────────────────────────────


def test_int32_roundtrip():
    data = np.array([0, 1, -1, 1000000, -999999], dtype=np.int32)
    ca = CompressedArray.from_int32(data)
    result = ca.to_numpy()
    np.testing.assert_array_equal(result, data)
    assert result.dtype == np.int32


# ── uint8 2-D round-trip ────────────────────────────────────────


def test_uint8_2d_roundtrip():
    data = np.zeros((128, 256), dtype=np.uint8)
    data[10:50, 20:80] = 255
    data[60:90, 100:200] = 128
    ca = CompressedArray.from_uint8_2d(data)
    result = ca.to_numpy()
    assert result.shape == (128, 256)
    np.testing.assert_array_equal(result, data)
    assert result.dtype == np.uint8


def test_uint8_2d_compression_sparse():
    data = np.zeros((1000, 1000), dtype=np.uint8)
    data[0, 0] = 255
    ca = CompressedArray.from_uint8_2d(data)
    assert ca.ratio < 0.01
    np.testing.assert_array_equal(ca.to_numpy(), data)


def test_uint8_2d_compression_uniform():
    data = np.full((500, 500), 200, dtype=np.uint8)
    ca = CompressedArray.from_uint8_2d(data)
    assert ca.ratio < 0.1


# ── edge cases ──────────────────────────────────────────────────


def test_empty_float32():
    data = np.array([], dtype=np.float32)
    ca = CompressedArray.from_float32(data)
    result = ca.to_numpy()
    assert result.size == 0


def test_single_element():
    data = np.array([42.0], dtype=np.float32)
    ca = CompressedArray.from_float32(data)
    np.testing.assert_array_equal(ca.to_numpy(), data)


# ── properties ──────────────────────────────────────────────────


def test_compressed_size():
    data = np.zeros(100_000, dtype=np.float32)
    ca = CompressedArray.from_float32(data)
    assert ca.compressed_size < ca.uncompressed_size
    assert ca.uncompressed_size == 100_000 * 4


def test_ratio_range():
    data = np.zeros(10_000, dtype=np.float32)
    ca = CompressedArray.from_float32(data)
    assert 0.0 < ca.ratio < 1.0


def test_ratio_uncompressed():
    data = np.array([1.0], dtype=np.float32)
    ca = CompressedArray.from_float32(data)
    assert ca.ratio == 1.0


# ── data integrity ──────────────────────────────────────────────


def test_random_float32_integrity():
    rng = np.random.default_rng(123)
    data = rng.standard_normal(50_000).astype(np.float32)
    ca = CompressedArray.from_float32(data)
    np.testing.assert_array_equal(ca.to_numpy(), data)


def test_random_uint8_2d_integrity():
    rng = np.random.default_rng(456)
    data = rng.integers(0, 256, (800, 600)).astype(np.uint8)
    ca = CompressedArray.from_uint8_2d(data)
    np.testing.assert_array_equal(ca.to_numpy(), data)


def test_multiple_decompressions_consistent():
    data = np.arange(1000, dtype=np.float32) * 0.1
    ca = CompressedArray.from_float32(data)
    a1 = ca.to_numpy()
    a2 = ca.to_numpy()
    np.testing.assert_array_equal(a1, a2)

"""Tests for ``raygeo.image.ramp.edge_ramp``.

The ramp is the distance from each burned pixel to the edge of the
burned area, as a fraction of the ramp length: 0 at the edge, 1 at the
ramp length and beyond, and 0 for pixels that are not burned. The edge
lies half a pixel outside the centre of the last burned pixel, and
everything outside the image counts as not burned.
"""

import numpy as np
import pytest

from raygeo.image.ramp import edge_ramp


def _square(size=21, margin=5):
    binary = np.zeros((size + 2 * margin, size + 2 * margin), np.uint8)
    binary[margin:-margin, margin:-margin] = 1
    return binary


def test_zero_length_keeps_the_binary_image():
    binary = _square()
    ramp = edge_ramp(binary, 0.0, (10.0, 10.0))
    assert ramp.dtype == np.float32
    np.testing.assert_array_equal(ramp, binary.astype(np.float32))


def test_unburned_pixels_stay_zero():
    binary = _square()
    ramp = edge_ramp(binary, 0.5, (10.0, 10.0))
    assert np.all(ramp[binary == 0] == 0.0)


def test_values_rise_linearly_from_the_edge():
    binary = _square(size=21, margin=5)
    ramp = edge_ramp(binary, 0.5, (10.0, 10.0))
    row = ramp[15, 5:16]
    expected = np.minimum(1.0, (np.arange(11) + 0.5) * 0.1 / 0.5)
    np.testing.assert_allclose(row, expected, atol=1e-6)


def test_centre_of_a_wide_area_is_at_full_level():
    ramp = edge_ramp(_square(size=21), 0.5, (10.0, 10.0))
    assert ramp[15, 15] == pytest.approx(1.0)


def test_distance_is_euclidean():
    binary = np.ones((21, 21), np.uint8)
    binary[10, 10] = 0
    ramp = edge_ramp(binary, 1.0, (10.0, 10.0))
    centre_distance = np.hypot(3, 4) * 0.1
    inside_unburned_pixel = 0.05 / 0.4
    assert ramp[13, 14] == pytest.approx(
        centre_distance * (1 - inside_unburned_pixel)
    )


def test_pixel_sizes_are_respected():
    binary = np.ones((9, 21), np.uint8)
    ramp = edge_ramp(binary, 10.0, (10.0, 2.0))
    assert ramp[4, 0] == pytest.approx(0.05 / 10.0)
    assert ramp[0, 10] == pytest.approx(0.25 / 10.0)


def test_outside_the_image_counts_as_not_burned():
    binary = np.ones((11, 11), np.uint8)
    ramp = edge_ramp(binary, 0.5, (10.0, 10.0))
    assert ramp[5, 0] == pytest.approx(0.1)
    assert ramp[5, 5] == pytest.approx(1.0)


def test_thin_strokes_do_not_reach_full_level():
    binary = np.zeros((10, 20), np.uint8)
    binary[:, 9:12] = 1
    ramp = edge_ramp(binary, 1.0, (10.0, 10.0))
    assert ramp[5, 10] == pytest.approx(0.15)
    assert ramp[5, 9] == pytest.approx(0.05)


def test_nonzero_values_count_as_burned():
    binary = np.zeros((5, 5), np.uint8)
    binary[2, 2] = 255
    ramp = edge_ramp(binary, 1.0, (10.0, 10.0))
    assert ramp[2, 2] == pytest.approx(0.05)


def test_rejects_invalid_arguments():
    binary = _square()
    with pytest.raises(ValueError):
        edge_ramp(binary, -1.0, (10.0, 10.0))
    with pytest.raises(ValueError):
        edge_ramp(binary, 1.0, (0.0, 10.0))

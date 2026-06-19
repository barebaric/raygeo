import numpy as np

from raygeo.image import (
    compute_adaptive_threshold,
    denoise_binary,
    filter_components,
    get_component_areas,
    grayscale_to_binary,
)

# ---------------------------------------------------------------------------
# grayscale_to_binary
# ---------------------------------------------------------------------------


class TestGrayscaleToBinary:
    def test_all_black_becomes_foreground(self):
        gray = np.full((10, 10), 0, dtype=np.uint8)
        result = grayscale_to_binary(gray, auto_threshold=False, threshold=0.5)
        assert np.all(result == 1)

    def test_all_white_becomes_background(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        result = grayscale_to_binary(gray, auto_threshold=False, threshold=0.5)
        assert np.all(result == 0)

    def test_fixed_threshold(self):
        gray = np.array([[0, 128, 255]], dtype=np.uint8)
        result = grayscale_to_binary(gray, auto_threshold=False, threshold=0.5)
        np.testing.assert_array_equal(result, [[1, 0, 0]])

    def test_invert_switches_foreground(self):
        gray = np.array([[0, 128, 255]], dtype=np.uint8)
        result = grayscale_to_binary(
            gray, auto_threshold=False, threshold=0.5, invert=True
        )
        np.testing.assert_array_equal(result, [[0, 1, 1]])

    def test_otsu_on_bimodal(self):
        # Two clear peaks: dark left half, bright right half
        gray = np.zeros((10, 20), dtype=np.uint8)
        gray[:, :10] = 0
        gray[:, 10:] = 200
        result = grayscale_to_binary(gray, auto_threshold=True)
        assert np.all(result[:, :10] == 1)
        assert np.all(result[:, 10:] == 0)

    def test_output_is_binary(self):
        gray = np.random.randint(0, 256, (20, 20), dtype=np.uint8)
        result = grayscale_to_binary(gray, auto_threshold=True)
        assert set(np.unique(result)).issubset({0, 1})

    def test_output_shape_preserved(self):
        gray = np.random.randint(0, 256, (15, 25), dtype=np.uint8)
        result = grayscale_to_binary(gray, auto_threshold=True)
        assert result.shape == (15, 25)

    def test_otsu_on_uniform_does_not_crash(self):
        gray = np.full((10, 10), 128, dtype=np.uint8)
        result = grayscale_to_binary(gray, auto_threshold=True)
        assert result.shape == (10, 10)


# ---------------------------------------------------------------------------
# get_component_areas
# ---------------------------------------------------------------------------


class TestGetComponentAreas:
    def test_single_component(self):
        binary = np.array([[0, 0, 0], [0, 1, 0], [0, 0, 0]], dtype=np.uint8)
        areas = get_component_areas(binary)
        assert areas == [1]

    def test_two_components(self):
        binary = np.array(
            [[1, 1, 0, 0], [1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 1, 1]],
            dtype=np.uint8,
        )
        areas = get_component_areas(binary)
        assert sorted(areas) == [2, 4]

    def test_empty_image(self):
        binary = np.zeros((10, 10), dtype=np.uint8)
        areas = get_component_areas(binary)
        assert len(areas) == 0

    def test_all_foreground_single_component(self):
        binary = np.ones((5, 5), dtype=np.uint8)
        areas = get_component_areas(binary)
        assert areas == [25]

    def test_areas_are_sorted(self):
        binary = np.array(
            [
                [1, 0, 1, 0, 1],
                [0, 0, 0, 0, 0],
                [1, 0, 0, 0, 1],
            ],
            dtype=np.uint8,
        )
        areas = get_component_areas(binary)
        assert areas == sorted(areas)


# ---------------------------------------------------------------------------
# filter_components
# ---------------------------------------------------------------------------


class TestFilterComponents:
    def test_removes_small_components(self):
        binary = np.array(
            [
                [0, 1, 1, 0],
                [0, 1, 1, 0],
                [0, 0, 0, 0],
                [0, 0, 1, 1],
            ],
            dtype=np.uint8,
        )
        filtered = filter_components(binary, min_area=3)
        # Only the 4-pixel top component should remain
        assert np.sum(filtered) == 4
        assert filtered[3, 2] == 0

    def test_keeps_large_components(self):
        binary = np.array(
            [
                [0, 1, 1, 0],
                [0, 1, 1, 0],
                [0, 0, 0, 0],
                [0, 0, 1, 1],
            ],
            dtype=np.uint8,
        )
        filtered = filter_components(binary, min_area=2)
        assert np.sum(filtered) == 6

    def test_empty_input_returns_empty(self):
        binary = np.zeros((5, 5), dtype=np.uint8)
        filtered = filter_components(binary, min_area=3)
        assert np.all(filtered == 0)

    def test_min_area_one_is_noop(self):
        binary = np.array([[1, 0, 1], [0, 0, 0], [1, 0, 1]], dtype=np.uint8)
        filtered = filter_components(binary, min_area=1)
        np.testing.assert_array_equal(filtered, binary)

    def test_output_shape_matches_input(self):
        binary = np.random.randint(0, 2, (16, 32), dtype=np.uint8)
        filtered = filter_components(binary, min_area=5)
        assert filtered.shape == (16, 32)


# ---------------------------------------------------------------------------
# denoise_binary
# ---------------------------------------------------------------------------


class TestComputeAdaptiveThreshold:
    def test_empty_areas_returns_zero(self):
        assert compute_adaptive_threshold([]) == 0

    def test_single_area_returns_two(self):
        assert compute_adaptive_threshold([50]) == 2

    def test_clean_heuristic_all_large_unique(self):
        areas = [100, 150, 200, 500]
        assert compute_adaptive_threshold(areas) == 2

    def test_noisy_with_gap_finds_threshold(self):
        # noise cluster around 1-10, then gap to 100
        areas = [1] * 100 + [2] * 50 + [10] * 5 + [100] * 2
        result = compute_adaptive_threshold(areas)
        assert result == 11

    def test_large_gap_in_middle(self):
        areas = [2, 2, 2, 50, 51]
        result = compute_adaptive_threshold(areas)
        assert result == 3

    def test_capped_at_100(self):
        areas = [1] * 10 + [200] * 5
        result = compute_adaptive_threshold(areas)
        assert result <= 100

    def test_minimum_return_is_two(self):
        areas = [1] * 100 + [2] * 100 + [3] * 100
        result = compute_adaptive_threshold(areas)
        assert result >= 2


class TestDenoiseBinary:
    def test_removes_small_noise(self):
        binary = np.zeros((10, 10), dtype=np.uint8)
        binary[2:8, 2:8] = 1  # 36px main component
        binary[0, 0] = 1  # 1px noise
        binary[9, 9] = 1  # 1px noise
        denoised = denoise_binary(binary)
        assert denoised[0, 0] == 0
        assert denoised[9, 9] == 0
        assert denoised[4, 4] == 1

    def test_preserves_large_content(self):
        binary = np.ones((50, 50), dtype=np.uint8)
        binary[0, 0] = 0  # one background pixel
        denoised = denoise_binary(binary)
        assert np.sum(denoised) >= 50 * 50 - 1

    def test_empty_input_returns_empty(self):
        binary = np.zeros((10, 10), dtype=np.uint8)
        denoised = denoise_binary(binary)
        assert np.all(denoised == 0)

    def test_multiple_noise_components(self):
        binary = np.zeros((100, 100), dtype=np.uint8)
        binary[25:75, 25:75] = 1  # 2500px main
        binary[5, 5] = 1  # 1px noise
        binary[10, 10:12] = 1  # 2px noise
        binary[90, 90:95] = 1  # 5px noise (could go either way)
        denoised = denoise_binary(binary)
        assert np.sum(denoised) >= 2500

    def test_shape_preserved(self):
        binary = np.random.randint(0, 2, (30, 40), dtype=np.uint8)
        denoised = denoise_binary(binary)
        assert denoised.shape == (30, 40)

    def test_output_is_binary(self):
        binary = np.random.randint(0, 2, (20, 20), dtype=np.uint8)
        denoised = denoise_binary(binary)
        assert set(np.unique(denoised)).issubset({0, 1})

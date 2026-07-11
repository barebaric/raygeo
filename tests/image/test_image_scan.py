import numpy as np
import pytest

from raygeo.image.scan import (
    ScanLine,
    ScanMode,
    downsample_power_values,
    find_mask_bounding_box,
    find_segments,
    generate_horizontal_scan_positions,
    generate_scan_lines,
    line_pixels,
    resample_rows,
)


class TestScanLine:
    def test_length_mm_horizontal(self):
        sl = ScanLine(0, (0.0, 0.0), (10.0, 0.0), [], 0.1)
        assert abs(sl.length_mm() - 10.0) < 1e-9

    def test_length_mm_vertical(self):
        sl = ScanLine(0, (0.0, 0.0), (0.0, 5.0), [], 0.1)
        assert abs(sl.length_mm() - 5.0) < 1e-9

    def test_length_mm_diagonal(self):
        sl = ScanLine(0, (0.0, 0.0), (3.0, 4.0), [], 0.1)
        assert abs(sl.length_mm() - 5.0) < 1e-9

    def test_direction_horizontal(self):
        sl = ScanLine(0, (0.0, 0.0), (10.0, 0.0), [], 0.1)
        dx, dy = sl.direction()
        assert abs(dx - 1.0) < 1e-9
        assert abs(dy) < 1e-9

    def test_direction_vertical(self):
        sl = ScanLine(0, (0.0, 0.0), (0.0, 10.0), [], 0.1)
        dx, dy = sl.direction()
        assert abs(dx) < 1e-9
        assert abs(dy - 1.0) < 1e-9

    def test_direction_zero_length(self):
        sl = ScanLine(0, (5.0, 5.0), (5.0, 5.0), [], 0.1)
        dx, dy = sl.direction()
        assert abs(dx - 1.0) < 1e-9
        assert abs(dy) < 1e-9

    def test_pixel_to_mm_horizontal(self):
        sl = ScanLine(0, (0.0, 0.5), (10.0, 0.5), [], 0.1)
        x, y = sl.pixel_to_mm(50, 5, (10.0, 10.0))
        assert abs(x - 5.0) < 0.01

    def test_pixel_to_mm_vertical(self):
        sl = ScanLine(0, (0.5, 0.0), (0.5, 10.0), [], 0.1)
        x, y = sl.pixel_to_mm(5, 50, (10.0, 10.0))
        assert abs(y - 5.0) < 0.01


class TestLinePixels:
    def test_horizontal_line(self):
        pixels = line_pixels((0.0, 5.0), (10.0, 5.0), 11, 11)
        assert len(pixels) == 11
        assert pixels[0] == (0, 5)
        assert pixels[10] == (10, 5)

    def test_vertical_line(self):
        pixels = line_pixels((5.0, 0.0), (5.0, 10.0), 11, 11)
        assert len(pixels) == 11
        assert pixels[0] == (5, 0)
        assert pixels[10] == (5, 10)

    def test_diagonal_line(self):
        pixels = line_pixels((0.0, 0.0), (10.0, 10.0), 11, 11)
        assert len(pixels) == 11
        assert pixels[0] == (0, 0)
        assert pixels[10] == (10, 10)

    def test_line_outside_bounds_clipped(self):
        pixels = line_pixels((-5.0, 5.0), (15.0, 5.0), 11, 11)
        for x, y in pixels:
            assert 0 <= x < 11
            assert 0 <= y < 11

    def test_single_pixel(self):
        pixels = line_pixels((5.0, 5.0), (5.0, 5.0), 11, 11)
        assert pixels == [(5, 5)]

    def test_empty_for_fully_outside(self):
        pixels = line_pixels((0.0, -5.0), (10.0, -5.0), 11, 11)
        assert len(pixels) == 0

    def test_reverse_horizontal(self):
        pixels = line_pixels((10.0, 5.0), (0.0, 5.0), 11, 11)
        assert len(pixels) == 11
        x_vals = [p[0] for p in pixels]
        assert sorted(set(x_vals)) == list(range(11))


class TestGenerateScanLines:
    @pytest.fixture
    def simple_bbox(self):
        return (0, 9, 0, 9)

    @pytest.fixture
    def simple_image_size(self):
        return (10, 10)

    @pytest.fixture
    def simple_ppm(self):
        return (10.0, 10.0)

    def test_horizontal_lines_count(
        self, simple_bbox, simple_image_size, simple_ppm
    ):
        lines = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 0.0, 0.0, 0.0
        )
        assert len(lines) >= 10

    def test_vertical_lines_count(
        self, simple_bbox, simple_image_size, simple_ppm
    ):
        lines = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 90.0, 0.0, 0.0
        )
        assert len(lines) >= 10

    def test_line_indices_are_sequential(
        self, simple_bbox, simple_image_size, simple_ppm
    ):
        lines = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 0.0, 0.0, 0.0
        )
        for i in range(1, len(lines)):
            assert lines[i].index == lines[i - 1].index + 1

    def test_horizontal_line_spacing(
        self, simple_bbox, simple_image_size, simple_ppm
    ):
        lines = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 0.0, 0.0, 0.0
        )
        if len(lines) >= 2:
            dy = abs(lines[1].start_mm[1] - lines[0].start_mm[1])
            assert abs(dy - 0.1) < 0.01

    def test_lines_contain_pixels(
        self, simple_bbox, simple_image_size, simple_ppm
    ):
        lines = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 0.0, 0.0, 0.0
        )
        for sl in lines:
            assert len(sl.pixels) > 0
            assert sl.pixels[0][0] >= 0

    def test_offset_alignment(
        self, simple_bbox, simple_image_size, simple_ppm
    ):
        lines = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 0.0, 0.0, 0.0
        )
        assert len(lines) > 0

    def test_offset_shifts_line_indices(
        self, simple_bbox, simple_image_size, simple_ppm
    ):
        lines_no_offset = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 0.0, 0.0, 0.0
        )
        lines_with_offset = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 0.0, 0.5, 0.0
        )
        if lines_no_offset and lines_with_offset:
            assert lines_no_offset[0].start_mm != lines_with_offset[0].start_mm

    def test_45_degree_lines(self, simple_bbox, simple_image_size, simple_ppm):
        lines = generate_scan_lines(
            simple_bbox, simple_image_size, simple_ppm, 0.1, 45.0, 0.0, 0.0
        )
        assert len(lines) > 0
        for sl in lines:
            assert len(sl.pixels) > 0

    def test_pixels_within_image_bounds(self):
        lines = generate_scan_lines(
            (0, 99, 0, 99), (100, 100), (10.0, 10.0), 0.1, 0.0, 0.0, 0.0
        )
        for sl in lines:
            for x, y in sl.pixels:
                assert 0 <= x < 100
                assert 0 <= y < 100

    def test_with_global_center(self):
        lines = generate_scan_lines(
            (0, 49, 0, 49),
            (50, 50),
            (10.0, 10.0),
            0.1,
            0.0,
            0.0,
            0.0,
            (0.0, 0.0),
        )
        assert len(lines) > 0
        for sl in lines:
            for x, y in sl.pixels:
                assert 0 <= x < 50
                assert 0 <= y < 50


class TestFindSegments:
    def test_empty(self):
        assert find_segments(np.array([], dtype=np.uint8)) == []

    def test_all_zeros(self):
        assert find_segments(np.array([0, 0, 0], dtype=np.uint8)) == []

    def test_all_ones(self):
        assert find_segments(np.array([1, 1, 1], dtype=np.uint8)) == [(0, 3)]

    def test_single(self):
        assert find_segments(np.array([0, 1, 0], dtype=np.uint8)) == [(1, 2)]

    def test_multiple(self):
        assert find_segments(np.array([0, 1, 1, 0, 1, 0], dtype=np.uint8)) == [
            (1, 3),
            (4, 5),
        ]

    def test_starts_with_value(self):
        assert find_segments(np.array([1, 1, 0, 0], dtype=np.uint8)) == [
            (0, 2)
        ]

    def test_ends_with_value(self):
        assert find_segments(np.array([0, 0, 1, 1], dtype=np.uint8)) == [
            (2, 4)
        ]

    def test_non_binary(self):
        assert find_segments(np.array([0, 5, 10, 0, 3], dtype=np.uint8)) == [
            (1, 3),
            (4, 5),
        ]

    def test_adjacent(self):
        assert find_segments(np.array([1, 0, 1, 1], dtype=np.uint8)) == [
            (0, 1),
            (2, 4),
        ]


class TestFindMaskBoundingBox:
    def test_all_white_returns_full(self):
        mask = np.full((10, 10), 255, dtype=np.uint8)
        assert find_mask_bounding_box(mask) == (0, 9, 0, 9)

    def test_all_zeros_returns_none(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        assert find_mask_bounding_box(mask) is None

    def test_single_pixel(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        mask[3, 7] = 1
        assert find_mask_bounding_box(mask) == (3, 3, 7, 7)

    def test_corner_region(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        mask[8:10, 8:10] = 1
        assert find_mask_bounding_box(mask) == (8, 9, 8, 9)

    def test_empty_mask_returns_none(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        assert find_mask_bounding_box(mask) is None


class TestGenerateHorizontalScanPositions:
    def test_basic_positions(self):
        mm, px = generate_horizontal_scan_positions(
            0, 9, 10, (10.0, 10.0), 0.1, 0.0
        )
        assert len(mm) > 0
        assert len(mm) == len(px)

    def test_offset_alignment(self):
        mm, px = generate_horizontal_scan_positions(
            0, 9, 10, (10.0, 10.0), 0.1, 0.5
        )
        assert len(mm) > 0
        for val in px:
            assert 0 <= val <= 9

    def test_empty_for_inverted_range(self):
        mm, px = generate_horizontal_scan_positions(
            20, 10, 30, (10.0, 10.0), 0.1, 0.0
        )
        assert len(mm) == 0
        assert len(px) == 0


class TestResampleRows:
    def test_integer_positions_identity(self):
        image = np.array([[10, 20, 30, 40], [50, 60, 70, 80]], dtype=np.uint8)
        y_coords = np.array([0.0, 1.0])
        result = resample_rows(image, y_coords)
        assert result.shape == (2, 4)
        np.testing.assert_array_almost_equal(result[0], [10, 20, 30, 40])

    def test_half_pixel_interpolation(self):
        image = np.array([[0, 0, 0, 0], [100, 100, 100, 100]], dtype=np.uint8)
        y_coords = np.array([0.5])
        result = resample_rows(image, y_coords)
        assert result.shape == (1, 4)
        expected = np.full((1, 4), 50.0)
        np.testing.assert_array_almost_equal(result, expected, decimal=0)

    def test_empty_y_coords(self):
        image = np.array([[1, 2], [3, 4]], dtype=np.uint8)
        y_coords = np.array([], dtype=np.float64)
        result = resample_rows(image, y_coords)
        assert result.shape == (0,) or result.shape[0] == 0


class TestScanMode:
    def test_import(self):
        assert ScanMode is not None

    def test_members(self):
        assert hasattr(ScanMode, "SEGMENTED")
        assert hasattr(ScanMode, "FULL_SWEEP")


class TestIntegration:
    def test_horizontal_scan_line_coverage(self):
        bbox = (0, 99, 0, 99)
        image_size = (100, 100)
        ppm = (10.0, 10.0)
        lines = generate_scan_lines(bbox, image_size, ppm, 0.1, 0.0, 0.0, 0.0)
        covered_rows = set()
        for sl in lines:
            for _, y in sl.pixels:
                covered_rows.add(y)
        assert len(covered_rows) > 50

    def test_segment_extraction_from_line(self):
        pixels = line_pixels((0.0, 5.0), (100.0, 5.0), 101, 11)
        values = np.ones(len(pixels), dtype=np.uint8)
        values[: len(values) // 3] = 0
        values[2 * len(values) // 3 :] = 0
        segments = find_segments(values)
        assert len(segments) == 1
        assert segments[0][0] > 0
        assert segments[0][1] < len(values)

    def test_scan_line_pixel_to_mm_roundtrip(self):
        sl = ScanLine(
            0,
            (0.0, 0.0),
            (10.0, 0.0),
            [(i, 0) for i in range(101)],
            0.1,
        )
        x, y = sl.pixel_to_mm(50, 0, (10.0, 10.0))
        assert abs(x - 5.0) < 0.1


class TestDownsamplePowerValues:
    def test_empty(self):
        p, x, y = downsample_power_values(
            np.array([], dtype=np.uint8), (0.0, 0.0), (1.0, 0.0), 0.1
        )
        assert len(p) == 0
        assert len(x) == 0
        assert len(y) == 0

    def test_single_value(self):
        p, x, y = downsample_power_values(
            np.array([128], dtype=np.uint8), (0.0, 0.0), (1.0, 0.0), 0.1
        )
        assert p[0] == 128
        assert x[0] == 0.0
        assert y[0] == 0.0

    def test_short_segment_passthrough(self):
        pv = np.array([100, 200], dtype=np.uint8)
        p, x, y = downsample_power_values(pv, (0.0, 0.0), (0.01, 0.0), 0.1)
        assert len(p) == 2
        assert p[0] == 100
        assert p[1] == 200

    def test_long_segment_downsamples(self):
        pv = np.arange(100, dtype=np.uint8)
        p, x, y = downsample_power_values(pv, (0.0, 0.0), (10.0, 0.0), 1.0)
        assert len(p) < len(pv)
        assert len(p) >= 2
        assert abs(x[0]) < 1e-9
        assert abs(x[-1] - 10.0) < 0.1

    def test_uniform_power(self):
        pv = np.full(50, 200, dtype=np.uint8)
        p, x, y = downsample_power_values(pv, (0.0, 0.0), (5.0, 0.0), 0.5)
        for val in p:
            assert val == 200


def test_generate_scan_lines():
    bbox = (0, 9, 0, 9)
    image_size = (10, 10)
    ppm = (10.0, 10.0)
    lines = generate_scan_lines(bbox, image_size, ppm, 0.1, 0.0, 0.0, 0.0)
    assert len(lines) >= 10

import numpy as np
import pytest

from raygeo.ops.raster import (
    ScanLine,
    ScanMode,
    downsample_power_values,
    find_mask_bounding_box,
    find_segments,
    generate_horizontal_scan_positions,
    generate_scan_lines,
    line_pixels,
    rasterize_mask_lines,
    rasterize_mask_scan,
    rasterize_multi_pass,
    rasterize_power_modulation,
    resample_rows,
)
from raygeo.ops.types import CommandType

# ---------------------------------------------------------------------------
# ScanLine
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# line_pixels
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# generate_scan_lines
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# find_segments
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# find_mask_bounding_box
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# generate_horizontal_scan_positions
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# resample_rows
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# Integration tests
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# Module import tests
# ---------------------------------------------------------------------------


class TestModuleImport:
    def test_import_from_raster(self):
        import raygeo.ops.raster as mod

        assert hasattr(mod, "ScanLine")
        assert hasattr(mod, "find_mask_bounding_box")
        assert hasattr(mod, "find_segments")
        assert hasattr(mod, "generate_horizontal_scan_positions")
        assert hasattr(mod, "generate_scan_lines")
        assert hasattr(mod, "line_pixels")
        assert hasattr(mod, "resample_rows")

    def test_scanline_is_class(self):
        sl = ScanLine(0, (0.0, 0.0), (1.0, 1.0), [(0, 0), (1, 1)], 0.1)
        assert sl.index == 0
        assert sl.start_mm == (0.0, 0.0)
        assert sl.end_mm == (1.0, 1.0)
        assert sl.pixels == [(0, 0), (1, 1)]
        assert sl.line_interval_mm == 0.1


# ---------------------------------------------------------------------------
# downsample_power_values
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# rasterize_power_modulation
# ---------------------------------------------------------------------------


class TestRasterizePowerModulation:
    def test_empty_alpha(self):
        gray = np.full((10, 10), 128, dtype=np.uint8)
        alpha = np.zeros((10, 10), dtype=np.uint8)
        ops = rasterize_power_modulation(
            gray, alpha, (10.0, 10.0), 0.0, 0.0, 0.1, 0.05
        )
        assert ops.is_empty()

    def test_full_image(self):
        gray = np.full((10, 10), 128, dtype=np.uint8)
        alpha = np.full((10, 10), 255, dtype=np.uint8)
        ops = rasterize_power_modulation(
            gray, alpha, (10.0, 10.0), 0.0, 0.0, 0.1, 0.05
        )
        assert not ops.is_empty()

    def test_white_image_empty(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        alpha = np.full((10, 10), 255, dtype=np.uint8)
        ops = rasterize_power_modulation(
            gray,
            alpha,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.05,
            min_power=0.0,
            max_power=1.0,
        )
        assert ops.is_empty()

    def test_with_power_quantization(self):
        gray = np.full((10, 10), 64, dtype=np.uint8)
        alpha = np.full((10, 10), 255, dtype=np.uint8)
        ops = rasterize_power_modulation(
            gray,
            alpha,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.05,
            num_power_levels=4,
        )
        assert not ops.is_empty()

    def test_with_angle(self):
        gray = np.full((20, 20), 128, dtype=np.uint8)
        alpha = np.full((20, 20), 255, dtype=np.uint8)
        ops = rasterize_power_modulation(
            gray, alpha, (10.0, 10.0), 0.0, 0.0, 0.1, 0.05, angle=45.0
        )
        assert not ops.is_empty()


# ---------------------------------------------------------------------------
# rasterize_mask_scan
# ---------------------------------------------------------------------------


class TestRasterizeMaskScan:
    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = rasterize_mask_scan(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
        assert ops.is_empty()

    def test_full_mask(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        ops = rasterize_mask_scan(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
        assert not ops.is_empty()

    def test_step_power(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        ops = rasterize_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.1, step_power=0.5
        )
        assert not ops.is_empty()

    def test_with_angle(self):
        mask = np.ones((20, 20), dtype=np.uint8)
        ops = rasterize_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.1, angle=90.0
        )
        assert not ops.is_empty()


# ---------------------------------------------------------------------------
# rasterize_mask_lines
# ---------------------------------------------------------------------------


class TestRasterizeMaskLines:
    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = rasterize_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
        assert ops.is_empty()

    def test_full_mask(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        ops = rasterize_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
        assert not ops.is_empty()

    def test_with_z_offset(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        ops = rasterize_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1, z=-2.0)
        assert not ops.is_empty()


# ---------------------------------------------------------------------------
# rasterize_multi_pass
# ---------------------------------------------------------------------------


class TestRasterizeMultiPass:
    def test_white_image_empty(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        ops = rasterize_multi_pass(gray, (10.0, 10.0), 0.0, 0.0, 0.1, 5, 0.5)
        assert ops.is_empty()

    def test_dark_image(self):
        gray = np.full((10, 10), 0, dtype=np.uint8)
        ops = rasterize_multi_pass(gray, (10.0, 10.0), 0.0, 0.0, 0.1, 3, 0.5)
        assert not ops.is_empty()

    def test_gradient(self):
        gray = np.zeros((20, 20), dtype=np.uint8)
        for i in range(20):
            gray[i, :] = int(i * 255 / 19)
        ops = rasterize_multi_pass(gray, (10.0, 10.0), 0.0, 0.0, 0.1, 3, 0.5)
        assert not ops.is_empty()

    def test_with_angle_increment(self):
        gray = np.full((10, 10), 64, dtype=np.uint8)
        ops = rasterize_multi_pass(
            gray,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            3,
            0.5,
            angle=0.0,
            angle_increment=45.0,
        )
        assert not ops.is_empty()


# ---------------------------------------------------------------------------
# ScanMode import & enum
# ---------------------------------------------------------------------------


class TestScanMode:
    def test_import(self):
        assert ScanMode is not None

    def test_members(self):
        assert hasattr(ScanMode, "Segmented")
        assert hasattr(ScanMode, "FullSweep")


# ---------------------------------------------------------------------------
# Full Sweep – rasterize_power_modulation
# ---------------------------------------------------------------------------


class TestFullSweepPowerModulation:
    def _make_images(self, size=30, gray_val=64):
        gray = np.full((size, size), gray_val, dtype=np.uint8)
        alpha = np.full((size, size), 255, dtype=np.uint8)
        return gray, alpha

    def test_fewer_scans_than_segmented(self):
        gray, alpha = self._make_images()
        seg = rasterize_power_modulation(
            gray, alpha, (10.0, 10.0), 0, 0, 0.1, 0.05
        )
        fs = rasterize_power_modulation(
            gray,
            alpha,
            (10.0, 10.0),
            0,
            0,
            0.1,
            0.05,
            scan_mode=ScanMode.FullSweep,
        )
        assert fs.len() <= seg.len()

    def test_produces_scan_lines(self):
        gray, alpha = self._make_images()
        ops = rasterize_power_modulation(
            gray,
            alpha,
            (10.0, 10.0),
            0,
            0,
            0.1,
            0.05,
            scan_mode=ScanMode.FullSweep,
        )
        types = [ops.command_type(i) for i in range(ops.len())]
        assert CommandType.SCAN_LINE in types

    def test_empty_alpha(self):
        gray = np.full((10, 10), 128, dtype=np.uint8)
        alpha = np.zeros((10, 10), dtype=np.uint8)
        ops = rasterize_power_modulation(
            gray,
            alpha,
            (10.0, 10.0),
            0,
            0,
            0.1,
            0.05,
            scan_mode=ScanMode.FullSweep,
        )
        assert ops.is_empty()


# ---------------------------------------------------------------------------
# Full Sweep – rasterize_mask_scan
# ---------------------------------------------------------------------------


class TestFullSweepMaskScan:
    def test_fewer_scans_than_segmented(self):
        mask = np.ones((30, 30), dtype=np.uint8)
        seg = rasterize_mask_scan(mask, (10.0, 10.0), 0, 0, 0.1)
        fs = rasterize_mask_scan(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FullSweep
        )
        assert fs.len() <= seg.len()

    def test_produces_scan_lines(self):
        mask = np.ones((20, 20), dtype=np.uint8)
        ops = rasterize_mask_scan(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FullSweep
        )
        types = [ops.command_type(i) for i in range(ops.len())]
        assert CommandType.SCAN_LINE in types

    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = rasterize_mask_scan(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FullSweep
        )
        assert ops.is_empty()


# ---------------------------------------------------------------------------
# Full Sweep – rasterize_mask_lines
# ---------------------------------------------------------------------------


class TestFullSweepMaskLines:
    def test_fewer_lines_than_segmented(self):
        mask = np.ones((30, 30), dtype=np.uint8)
        seg = rasterize_mask_lines(mask, (10.0, 10.0), 0, 0, 0.1)
        fs = rasterize_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FullSweep
        )
        assert fs.len() <= seg.len()

    def test_produces_lines(self):
        mask = np.ones((20, 20), dtype=np.uint8)
        ops = rasterize_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FullSweep
        )
        types = [ops.command_type(i) for i in range(ops.len())]
        assert CommandType.LINE_TO in types

    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = rasterize_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FullSweep
        )
        assert ops.is_empty()


# ---------------------------------------------------------------------------
# Full Sweep – rasterize_multi_pass
# ---------------------------------------------------------------------------


class TestFullSweepMultiPass:
    def test_fewer_lines_than_segmented(self):
        gray = np.full((20, 20), 64, dtype=np.uint8)
        seg = rasterize_multi_pass(gray, (10.0, 10.0), 0, 0, 0.1, 3, 0.5)
        fs = rasterize_multi_pass(
            gray,
            (10.0, 10.0),
            0,
            0,
            0.1,
            3,
            0.5,
            scan_mode=ScanMode.FullSweep,
        )
        assert fs.len() <= seg.len()

    def test_produces_lines(self):
        gray = np.full((20, 20), 64, dtype=np.uint8)
        ops = rasterize_multi_pass(
            gray,
            (10.0, 10.0),
            0,
            0,
            0.1,
            3,
            0.5,
            scan_mode=ScanMode.FullSweep,
        )
        assert not ops.is_empty()

    def test_white_image_empty(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        ops = rasterize_multi_pass(
            gray,
            (10.0, 10.0),
            0,
            0,
            0.1,
            3,
            0.5,
            scan_mode=ScanMode.FullSweep,
        )
        assert ops.is_empty()

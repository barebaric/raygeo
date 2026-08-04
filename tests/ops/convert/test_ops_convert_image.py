import numpy as np

from raygeo.image.scan import ScanMode
from raygeo.ops import Ops
from raygeo.ops.types import CommandType


class TestFromPowerModulatedImage:
    def test_empty_alpha(self):
        gray = np.full((10, 10), 128, dtype=np.uint8)
        alpha = np.zeros((10, 10), dtype=np.uint8)
        ops = Ops.from_power_modulated_image(
            gray, alpha, (10.0, 10.0), 0.0, 0.0, 0.1, 0.05
        )
        assert ops.is_empty()

    def test_full_image(self):
        gray = np.full((10, 10), 128, dtype=np.uint8)
        alpha = np.full((10, 10), 255, dtype=np.uint8)
        ops = Ops.from_power_modulated_image(
            gray, alpha, (10.0, 10.0), 0.0, 0.0, 0.1, 0.05
        )
        assert not ops.is_empty()

    def test_white_image_empty(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        alpha = np.full((10, 10), 255, dtype=np.uint8)
        ops = Ops.from_power_modulated_image(
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
        ops = Ops.from_power_modulated_image(
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
        ops = Ops.from_power_modulated_image(
            gray, alpha, (10.0, 10.0), 0.0, 0.0, 0.1, 0.05, angle=45.0
        )
        assert not ops.is_empty()


class TestFromMaskScan:
    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_scan(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
        assert ops.is_empty()

    def test_full_mask(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_scan(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
        assert not ops.is_empty()

    def test_step_power(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.1, step_power=0.5
        )
        assert not ops.is_empty()

    def test_with_angle(self):
        mask = np.ones((20, 20), dtype=np.uint8)
        ops = Ops.from_mask_scan(mask, (10.0, 10.0), 0.0, 0.0, 0.1, angle=90.0)
        assert not ops.is_empty()


class TestFromMaskLines:
    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
        assert ops.is_empty()

    def test_full_mask(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
        assert not ops.is_empty()

    def test_with_z_offset(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1, z=-2.0)
        assert not ops.is_empty()


class TestFromMultiPassImage:
    def test_white_image_empty(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        ops = Ops.from_multi_pass_image(
            gray, (10.0, 10.0), 0.0, 0.0, 0.1, 5, 0.5
        )
        assert ops.is_empty()

    def test_dark_image(self):
        gray = np.full((10, 10), 0, dtype=np.uint8)
        ops = Ops.from_multi_pass_image(
            gray, (10.0, 10.0), 0.0, 0.0, 0.1, 3, 0.5
        )
        assert not ops.is_empty()

    def test_gradient(self):
        gray = np.zeros((20, 20), dtype=np.uint8)
        for i in range(20):
            gray[i, :] = int(i * 255 / 19)
        ops = Ops.from_multi_pass_image(
            gray, (10.0, 10.0), 0.0, 0.0, 0.1, 3, 0.5
        )
        assert not ops.is_empty()

    def test_with_angle_increment(self):
        gray = np.full((10, 10), 64, dtype=np.uint8)
        ops = Ops.from_multi_pass_image(
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


class TestFullSweepPowerModulation:
    def _make_images(self, size=30, gray_val=64):
        gray = np.full((size, size), gray_val, dtype=np.uint8)
        alpha = np.full((size, size), 255, dtype=np.uint8)
        return gray, alpha

    def test_fewer_scans_than_segmented(self):
        gray, alpha = self._make_images()
        seg = Ops.from_power_modulated_image(
            gray, alpha, (10.0, 10.0), 0, 0, 0.1, 0.05
        )
        fs = Ops.from_power_modulated_image(
            gray,
            alpha,
            (10.0, 10.0),
            0,
            0,
            0.1,
            0.05,
            scan_mode=ScanMode.FULL_SWEEP,
        )
        assert fs.len() <= seg.len()

    def test_produces_scan_lines(self):
        gray, alpha = self._make_images()
        ops = Ops.from_power_modulated_image(
            gray,
            alpha,
            (10.0, 10.0),
            0,
            0,
            0.1,
            0.05,
            scan_mode=ScanMode.FULL_SWEEP,
        )
        types = [ops.command_type(i) for i in range(ops.len())]
        assert CommandType.SCAN_LINE in types

    def test_empty_alpha(self):
        gray = np.full((10, 10), 128, dtype=np.uint8)
        alpha = np.zeros((10, 10), dtype=np.uint8)
        ops = Ops.from_power_modulated_image(
            gray,
            alpha,
            (10.0, 10.0),
            0,
            0,
            0.1,
            0.05,
            scan_mode=ScanMode.FULL_SWEEP,
        )
        assert ops.is_empty()


class TestFullSweepMaskScan:
    def test_fewer_scans_than_segmented(self):
        mask = np.ones((30, 30), dtype=np.uint8)
        seg = Ops.from_mask_scan(mask, (10.0, 10.0), 0, 0, 0.1)
        fs = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        assert fs.len() <= seg.len()

    def test_produces_scan_lines(self):
        mask = np.ones((20, 20), dtype=np.uint8)
        ops = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        types = [ops.command_type(i) for i in range(ops.len())]
        assert CommandType.SCAN_LINE in types

    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        assert ops.is_empty()


class TestFullSweepMaskLines:
    def test_fewer_commands_than_segmented(self):
        mask = np.zeros((30, 30), dtype=np.uint8)
        mask[5:25, 5:15] = 1
        mask[5:25, 18:25] = 1
        seg = Ops.from_mask_lines(mask, (10.0, 10.0), 0, 0, 0.1)
        fs = Ops.from_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        assert fs.len() < seg.len()

    def test_produces_scanlines(self):
        mask = np.ones((20, 20), dtype=np.uint8)
        ops = Ops.from_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        types = [ops.command_type(i) for i in range(ops.len())]
        assert CommandType.SCAN_LINE in types

    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        assert ops.is_empty()

    def test_power_zero_in_gaps(self):
        mask = np.zeros((30, 30), dtype=np.uint8)
        mask[5:25, 5:25] = 1
        ops = Ops.from_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) > 0
        for i in scan_indices:
            pv = ops.scanline_data(i)
            assert min(pv) == 0
            assert max(pv) > 0

    def test_one_scanline_per_scan_row(self):
        mask = np.zeros((30, 30), dtype=np.uint8)
        mask[5:25, 5:15] = 1
        mask[5:25, 18:25] = 1
        ops = Ops.from_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        scan_count = sum(
            1
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        )
        seg = Ops.from_mask_lines(mask, (10.0, 10.0), 0, 0, 0.1)
        line_count = sum(
            1
            for i in range(seg.len())
            if seg.command_type(i) == CommandType.LINE_TO
        )
        assert scan_count < line_count


class TestFullSweepMultiPass:
    def test_fewer_commands_than_segmented(self):
        gray = np.full((20, 20), 255, dtype=np.uint8)
        gray[5:15, 5:10] = 64
        gray[5:15, 12:15] = 64
        seg = Ops.from_multi_pass_image(gray, (10.0, 10.0), 0, 0, 0.1, 3, 0.5)
        fs = Ops.from_multi_pass_image(
            gray,
            (10.0, 10.0),
            0,
            0,
            0.1,
            3,
            0.5,
            scan_mode=ScanMode.FULL_SWEEP,
        )
        assert fs.len() < seg.len()

    def test_produces_scanlines(self):
        gray = np.full((20, 20), 64, dtype=np.uint8)
        ops = Ops.from_multi_pass_image(
            gray,
            (10.0, 10.0),
            0,
            0,
            0.1,
            3,
            0.5,
            scan_mode=ScanMode.FULL_SWEEP,
        )
        types = [ops.command_type(i) for i in range(ops.len())]
        assert CommandType.SCAN_LINE in types

    def test_white_image_empty(self):
        gray = np.full((10, 10), 255, dtype=np.uint8)
        ops = Ops.from_multi_pass_image(
            gray,
            (10.0, 10.0),
            0,
            0,
            0.1,
            3,
            0.5,
            scan_mode=ScanMode.FULL_SWEEP,
        )
        assert ops.is_empty()


def test_from_mask_lines():
    mask = np.ones((10, 10), dtype=np.uint8)
    ops = Ops.from_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
    assert not ops.is_empty()


def test_from_mask_lines_empty():
    mask = np.zeros((10, 10), dtype=np.uint8)
    ops = Ops.from_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
    assert ops.is_empty()


def test_scan_mode_enum():
    assert ScanMode.SEGMENTED is not None
    assert ScanMode.FULL_SWEEP is not None


class TestDotWidthCorrection:
    """Must never change toolpath geometry, only which power samples fire."""

    def _endpoints(self, ops):
        return [ops.endpoint(i) for i in range(ops.len())]

    def _command_types(self, ops):
        return [ops.command_type(i) for i in range(ops.len())]

    def test_mask_scan_geometry_unchanged_segmented(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        baseline = Ops.from_mask_scan(mask, (10.0, 10.0), 0.0, 0.0, 0.1, 1.0)
        trimmed = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.1, 1.0, dot_width_correction_mm=0.2
        )
        assert self._command_types(baseline) == self._command_types(trimmed)
        for a, b in zip(self._endpoints(baseline), self._endpoints(trimmed)):
            assert a == b

    def test_mask_scan_geometry_unchanged_full_sweep(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        baseline = Ops.from_mask_scan(
            mask,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            1.0,
            scan_mode=ScanMode.FULL_SWEEP,
        )
        trimmed = Ops.from_mask_scan(
            mask,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            1.0,
            scan_mode=ScanMode.FULL_SWEEP,
            dot_width_correction_mm=0.2,
        )
        assert self._command_types(baseline) == self._command_types(trimmed)
        for a, b in zip(self._endpoints(baseline), self._endpoints(trimmed)):
            assert a == b

    def test_mask_scan_trims_power_at_each_end(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        # 10 px/mm, 0.2mm correction -> trim 2 samples off each end.
        trimmed = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.1, 1.0, dot_width_correction_mm=0.2
        )
        data = trimmed.scanline_data(1)
        assert list(data[:2]) == [0, 0]
        assert list(data[-2:]) == [0, 0]
        assert all(v > 0 for v in data[2:-2])

    def test_power_modulated_geometry_unchanged(self):
        gray = np.full((10, 10), 0, dtype=np.uint8)
        alpha = np.full((10, 10), 255, dtype=np.uint8)
        baseline = Ops.from_power_modulated_image(
            gray, alpha, (10.0, 10.0), 0.0, 0.0, 0.1, 0.02
        )
        trimmed = Ops.from_power_modulated_image(
            gray,
            alpha,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.02,
            dot_width_correction_mm=0.2,
        )
        assert self._command_types(baseline) == self._command_types(trimmed)
        for a, b in zip(self._endpoints(baseline), self._endpoints(trimmed)):
            assert a == b

    def test_power_modulated_trims_power_at_each_end(self):
        gray = np.full((10, 10), 0, dtype=np.uint8)
        alpha = np.full((10, 10), 255, dtype=np.uint8)
        trimmed = Ops.from_power_modulated_image(
            gray,
            alpha,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.02,
            dot_width_correction_mm=0.2,
        )
        data = trimmed.scanline_data(1)
        assert list(data[:2]) == [0, 0]
        assert list(data[-2:]) == [0, 0]

    def test_zero_correction_is_no_op(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        baseline = Ops.from_mask_scan(mask, (10.0, 10.0), 0.0, 0.0, 0.1, 1.0)
        explicit_zero = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.1, 1.0, dot_width_correction_mm=0.0
        )
        assert baseline.scanline_data(1) == explicit_zero.scanline_data(1)

    def test_trim_larger_than_segment_zeroes_whole_run(self):
        mask = np.ones((10, 10), dtype=np.uint8)
        trimmed = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.1, 1.0, dot_width_correction_mm=5.0
        )
        data = trimmed.scanline_data(1)
        assert all(v == 0 for v in data)

    def test_geometry_unchanged_at_nonzero_angle(self):
        mask = np.ones((20, 20), dtype=np.uint8)
        baseline = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.2, 1.0, angle=30.0
        )
        trimmed = Ops.from_mask_scan(
            mask,
            (10.0, 10.0),
            0.0,
            0.0,
            0.2,
            1.0,
            angle=30.0,
            dot_width_correction_mm=0.2,
        )
        assert self._command_types(baseline) == self._command_types(trimmed)
        for a, b in zip(self._endpoints(baseline), self._endpoints(trimmed)):
            assert a == b

        scan_indices = [
            i
            for i, ct in enumerate(self._command_types(trimmed))
            if ct == CommandType.SCAN_LINE
        ]
        assert scan_indices
        # Pick the longest line; corner ones can be fully trimmed away.
        data = max((trimmed.scanline_data(i) for i in scan_indices), key=len)
        assert len(data) > 4
        assert data[0] == 0
        assert data[-1] == 0
        assert any(v > 0 for v in data)

    def test_multiple_segments_trimmed_independently(self):
        mask = np.zeros((20, 20), dtype=np.uint8)
        mask[:, 0:8] = 1
        mask[:, 12:20] = 1

        trimmed = Ops.from_mask_scan(
            mask, (10.0, 10.0), 0.0, 0.0, 0.2, 1.0, dot_width_correction_mm=0.1
        )

        scan_indices = [
            i
            for i in range(trimmed.len())
            if trimmed.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(scan_indices) >= 2

        first_seg = trimmed.scanline_data(scan_indices[0])
        second_seg = trimmed.scanline_data(scan_indices[1])
        for seg in (first_seg, second_seg):
            assert seg[0] == 0
            assert seg[-1] == 0
            assert any(v > 0 for v in seg)

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
    def test_fewer_lines_than_segmented(self):
        mask = np.ones((30, 30), dtype=np.uint8)
        seg = Ops.from_mask_lines(mask, (10.0, 10.0), 0, 0, 0.1)
        fs = Ops.from_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        assert fs.len() <= seg.len()

    def test_produces_lines(self):
        mask = np.ones((20, 20), dtype=np.uint8)
        ops = Ops.from_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        types = [ops.command_type(i) for i in range(ops.len())]
        assert CommandType.LINE_TO in types

    def test_empty_mask(self):
        mask = np.zeros((10, 10), dtype=np.uint8)
        ops = Ops.from_mask_lines(
            mask, (10.0, 10.0), 0, 0, 0.1, scan_mode=ScanMode.FULL_SWEEP
        )
        assert ops.is_empty()


class TestFullSweepMultiPass:
    def test_fewer_lines_than_segmented(self):
        gray = np.full((20, 20), 64, dtype=np.uint8)
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
        assert fs.len() <= seg.len()

    def test_produces_lines(self):
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
        assert not ops.is_empty()

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

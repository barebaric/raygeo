"""Tests for the error-diffusion and screen dithering algorithms."""

import numpy as np
import pytest

from raygeo.image.dither import (
    apply_bayer_dither,
    apply_error_diffusion_dither,
    apply_floyd_steinberg_dither,
    apply_halftone_dither,
    apply_newsprint_dither,
)
from raygeo.image.srgb import srgb_to_linear

KERNELS = [
    "floyd_steinberg",
    "atkinson",
    "stucki",
    "jarvis_judice_ninke",
    "sierra",
    "sierra_2row",
    "sierra_lite",
    "burkes",
]

NEW_KERNELS = [k for k in KERNELS if k != "floyd_steinberg"]

REFERENCE_KERNELS = {
    "floyd_steinberg": (
        16,
        [(1, 0, 7), (-1, 1, 3), (0, 1, 5), (1, 1, 1)],
    ),
    "atkinson": (
        8,
        [(1, 0, 1), (2, 0, 1), (-1, 1, 1), (0, 1, 1), (1, 1, 1), (0, 2, 1)],
    ),
    "stucki": (
        42,
        [
            (1, 0, 8),
            (2, 0, 4),
            (-2, 1, 2),
            (-1, 1, 4),
            (0, 1, 8),
            (1, 1, 4),
            (2, 1, 2),
            (-2, 2, 1),
            (-1, 2, 2),
            (0, 2, 4),
            (1, 2, 2),
            (2, 2, 1),
        ],
    ),
    "jarvis_judice_ninke": (
        48,
        [
            (1, 0, 7),
            (2, 0, 5),
            (-2, 1, 3),
            (-1, 1, 5),
            (0, 1, 7),
            (1, 1, 5),
            (2, 1, 3),
            (-2, 2, 1),
            (-1, 2, 3),
            (0, 2, 5),
            (1, 2, 3),
            (2, 2, 1),
        ],
    ),
    "sierra": (
        32,
        [
            (1, 0, 5),
            (2, 0, 3),
            (-2, 1, 2),
            (-1, 1, 4),
            (0, 1, 5),
            (1, 1, 4),
            (2, 1, 2),
            (-1, 2, 2),
            (0, 2, 3),
            (1, 2, 2),
        ],
    ),
    "sierra_2row": (
        16,
        [
            (1, 0, 4),
            (2, 0, 3),
            (-2, 1, 1),
            (-1, 1, 2),
            (0, 1, 3),
            (1, 1, 2),
            (2, 1, 1),
        ],
    ),
    "sierra_lite": (4, [(1, 0, 2), (-1, 1, 1), (0, 1, 1)]),
    "burkes": (
        32,
        [
            (1, 0, 8),
            (2, 0, 4),
            (-2, 1, 2),
            (-1, 1, 4),
            (0, 1, 8),
            (1, 1, 4),
            (2, 1, 2),
        ],
    ),
}

BAYER8 = np.array(
    [
        [0, 32, 8, 40, 2, 34, 10, 42],
        [48, 16, 56, 24, 50, 18, 58, 26],
        [12, 44, 4, 36, 14, 46, 6, 38],
        [60, 28, 52, 20, 62, 30, 54, 22],
        [3, 35, 11, 43, 1, 33, 9, 41],
        [51, 19, 59, 27, 49, 17, 57, 25],
        [15, 47, 7, 39, 13, 45, 5, 37],
        [63, 31, 55, 23, 61, 29, 53, 21],
    ],
    dtype=np.float32,
)


def reference_diffusion(grayscale, kernel, invert, serpentine):
    """Straightforward per-pixel error diffusion used as an oracle."""
    divisor, taps = REFERENCE_KERNELS[kernel]
    forward = {
        dx: np.float32(w) / np.float32(divisor)
        for dx, dy, w in taps
        if dy == 0
    }
    later = [
        (dx, dy, np.float32(w) / np.float32(divisor))
        for dx, dy, w in taps
        if dy != 0
    ]
    height, width = grayscale.shape
    pad = max((abs(dx) for dx, _dy, _w in later), default=0)
    extra_rows = max((dy for _dx, dy, _w in later), default=0)
    work = np.zeros((height + extra_rows, width + 2 * pad), dtype=np.float32)
    work[:height, pad : pad + width] = srgb_to_linear(grayscale)
    out = np.zeros((height, width), dtype=np.uint8)
    for y in range(height):
        reverse = serpentine and y % 2 == 1
        row = work[y, pad : pad + width]
        if reverse:
            row = row[::-1]
        values = np.concatenate((row, np.zeros(2, dtype=np.float32)))
        errors = np.zeros(width, dtype=np.float32)
        dark = np.zeros(width, dtype=np.uint8)
        for x in range(width):
            old = values[x]
            if old < np.float32(0.5):
                dark[x] = 1
                errors[x] = old
            else:
                errors[x] = old - np.float32(1.0)
            values[x + 1] += errors[x] * forward[1]
            values[x + 2] += errors[x] * forward.get(2, np.float32(0.0))
        row_out = dark[::-1] if reverse else dark
        if invert:
            out[y] = 1 - row_out
        else:
            out[y] = row_out
        if reverse:
            errors = errors[::-1].copy()
        for dx, dy, weight in later:
            start = pad - dx if reverse else pad + dx
            work[y + dy, start : start + width] += errors * weight
    return out


def dot_boxes(binary):
    """Bounding boxes of the 4-connected dot clusters of ones."""
    height, width = binary.shape
    visited = np.zeros((height, width), dtype=bool)
    boxes = []
    for start_y, start_x in zip(*np.nonzero(binary)):
        if visited[start_y, start_x]:
            continue
        stack = [(start_y, start_x)]
        visited[start_y, start_x] = True
        min_y = max_y = start_y
        min_x = max_x = start_x
        while stack:
            y, x = stack.pop()
            min_y = min(min_y, y)
            max_y = max(max_y, y)
            min_x = min(min_x, x)
            max_x = max(max_x, x)
            for ny, nx in ((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)):
                if (
                    0 <= ny < height
                    and 0 <= nx < width
                    and binary[ny, nx]
                    and not visited[ny, nx]
                ):
                    visited[ny, nx] = True
                    stack.append((ny, nx))
        boxes.append((min_y, max_y, min_x, max_x))
    return boxes


class TestErrorDiffusionCore:
    @pytest.mark.parametrize("serpentine", [False, True])
    @pytest.mark.parametrize("kernel", KERNELS)
    def test_matches_reference_implementation(self, kernel, serpentine):
        rng = np.random.default_rng(7)
        for shape in [(1, 1), (1, 6), (5, 1), (4, 6), (6, 5)]:
            grayscale = rng.integers(0, 256, shape).astype(np.uint8)
            expected = reference_diffusion(
                grayscale, kernel, False, serpentine
            )
            result = apply_error_diffusion_dither(
                grayscale, kernel, serpentine=serpentine
            )
            np.testing.assert_array_equal(result, expected)

    def test_matches_legacy_floyd_steinberg(self):
        rng = np.random.default_rng(11)
        for invert in (False, True):
            grayscale = rng.integers(0, 256, (40, 60)).astype(np.uint8)
            legacy = apply_floyd_steinberg_dither(grayscale, invert)
            result = apply_error_diffusion_dither(
                grayscale, "floyd_steinberg", invert=invert
            )
            np.testing.assert_array_equal(result, legacy)

    def test_unknown_kernel_raises(self):
        grayscale = np.zeros((4, 4), dtype=np.uint8)
        with pytest.raises(ValueError, match="Unknown dither kernel"):
            apply_error_diffusion_dither(grayscale, "nope")

    def test_serpentine_reverses_odd_rows(self):
        grayscale = np.tile(np.linspace(0, 255, 8).astype(np.uint8), (4, 1))
        plain = apply_error_diffusion_dither(grayscale, "sierra_lite")
        serpentine = apply_error_diffusion_dither(
            grayscale, "sierra_lite", serpentine=True
        )
        np.testing.assert_array_equal(plain[0], serpentine[0])
        assert not np.array_equal(plain, serpentine)

    def test_does_not_modify_input(self):
        grayscale = np.full((5, 5), 77, dtype=np.uint8)
        original = grayscale.copy()
        apply_error_diffusion_dither(grayscale, "stucki", serpentine=True)
        np.testing.assert_array_equal(grayscale, original)

    @pytest.mark.parametrize("shape", [(0, 4), (4, 0)])
    def test_empty_image(self, shape):
        result = apply_error_diffusion_dither(
            np.zeros(shape, dtype=np.uint8), "stucki"
        )
        assert result.shape == shape


class TestErrorDiffusionBehaviour:
    @pytest.mark.parametrize("kernel", NEW_KERNELS)
    def test_white_is_never_engraved(self, kernel):
        white = np.full((24, 24), 255, dtype=np.uint8)
        result = apply_error_diffusion_dither(white, kernel)
        assert result.dtype == np.uint8
        assert not result.any()

    @pytest.mark.parametrize("kernel", NEW_KERNELS)
    def test_black_is_fully_engraved(self, kernel):
        black = np.zeros((24, 24), dtype=np.uint8)
        result = apply_error_diffusion_dither(black, kernel)
        assert result.all()

    @pytest.mark.parametrize("kernel", NEW_KERNELS)
    def test_output_is_binary_and_deterministic(self, kernel):
        rng = np.random.default_rng(3)
        grayscale = rng.integers(0, 256, (31, 47)).astype(np.uint8)
        first = apply_error_diffusion_dither(grayscale, kernel)
        second = apply_error_diffusion_dither(grayscale, kernel)
        assert set(np.unique(first)) <= {0, 1}
        np.testing.assert_array_equal(first, second)

    @pytest.mark.parametrize("kernel", KERNELS)
    def test_invert_is_the_complement(self, kernel):
        rng = np.random.default_rng(5)
        grayscale = rng.integers(0, 256, (20, 30)).astype(np.uint8)
        inverted = apply_error_diffusion_dither(grayscale, kernel, True)
        plain = apply_error_diffusion_dither(grayscale, kernel)
        np.testing.assert_array_equal(inverted, 1 - plain)

    @pytest.mark.parametrize("gray", [40, 128, 200])
    @pytest.mark.parametrize("kernel", [k for k in KERNELS if k != "atkinson"])
    def test_preserves_linear_tone(self, kernel, gray):
        grayscale = np.full((64, 96), gray, dtype=np.uint8)
        result = apply_error_diffusion_dither(grayscale, kernel)
        expected = 1.0 - float(
            srgb_to_linear(np.array([gray], dtype=np.uint8))[0]
        )
        assert result.mean() == pytest.approx(expected, abs=0.02)

    def test_atkinson_loses_a_quarter_of_the_error(self):
        """Atkinson only diffuses 6/8 of the error, which clips dark
        and light tones and raises contrast compared to Stucki."""
        grayscale = np.full((64, 64), 230, dtype=np.uint8)
        atkinson = apply_error_diffusion_dither(grayscale, "atkinson")
        stucki = apply_error_diffusion_dither(grayscale, "stucki")
        assert atkinson.mean() < stucki.mean()


class TestHalftone:
    @pytest.mark.parametrize("gray", [30, 100, 160, 220])
    def test_coverage_follows_darkness(self, gray):
        grayscale = np.full((200, 200), gray, dtype=np.uint8)
        result = apply_halftone_dither(
            grayscale, 1.0, 45.0, pixels_per_mm=(10.0, 10.0)
        )
        assert result.mean() == pytest.approx(1.0 - gray / 255.0, abs=0.03)

    def test_dot_count_follows_cell_size(self):
        """At 0 degrees the dot centres sit on whole cells from the
        origin, so a 20 mm square holds 21 x 21 (partly clipped) dots
        at 1 mm."""
        grayscale = np.full((200, 200), 230, dtype=np.uint8)
        for cell_mm, expected in ((1.0, 441), (2.0, 121)):
            result = apply_halftone_dither(
                grayscale,
                cell_mm,
                0.0,
                pixels_per_mm=(10.0, 10.0),
            )
            assert len(dot_boxes(result)) == pytest.approx(expected, rel=0.1)

    def test_dots_are_round_on_anisotropic_pixels(self):
        """X is sampled twice as densely as Y; the dots must still be
        round in millimetres, so twice as wide in pixels."""
        grayscale = np.full((100, 200), 220, dtype=np.uint8)
        result = apply_halftone_dither(
            grayscale, 2.0, 0.0, pixels_per_mm=(20.0, 10.0)
        )
        interior = [
            (min_y, max_y, min_x, max_x)
            for min_y, max_y, min_x, max_x in dot_boxes(result)
            if min_y > 0
            and min_x > 0
            and max_y < grayscale.shape[0] - 1
            and max_x < grayscale.shape[1] - 1
        ]
        assert interior
        for min_y, max_y, min_x, max_x in interior:
            height = max_y - min_y + 1
            width = max_x - min_x + 1
            assert width == pytest.approx(2 * height, abs=2)

    def test_angle_rotates_the_screen(self):
        grayscale = np.full((120, 120), 200, dtype=np.uint8)
        straight = apply_halftone_dither(
            grayscale, 1.0, 0.0, pixels_per_mm=(10.0, 10.0)
        )
        rotated = apply_halftone_dither(
            grayscale, 1.0, 45.0, pixels_per_mm=(10.0, 10.0)
        )
        assert not np.array_equal(straight, rotated)
        assert rotated.mean() == pytest.approx(straight.mean(), abs=0.03)

    def test_invalid_cell_size_falls_back_to_one_pixel_cells(self):
        grayscale = np.full((10, 10), 0, dtype=np.uint8)
        result = apply_halftone_dither(
            grayscale, 0.0, 0.0, pixels_per_mm=(10.0, 10.0)
        )
        assert result.all()

    def test_invert_is_the_complement(self):
        rng = np.random.default_rng(13)
        grayscale = rng.integers(0, 256, (40, 40)).astype(np.uint8)
        inverted = apply_halftone_dither(grayscale, 1.0, 30.0, invert=True)
        plain = apply_halftone_dither(grayscale, 1.0, 30.0)
        np.testing.assert_array_equal(inverted, 1 - plain)

    def test_non_positive_pixels_per_mm_raises(self):
        grayscale = np.zeros((4, 4), dtype=np.uint8)
        with pytest.raises(ValueError, match="pixels_per_mm"):
            apply_halftone_dither(
                grayscale, 1.0, 0.0, pixels_per_mm=(0.0, 1.0)
            )


class TestNewsprint:
    def test_white_is_never_engraved(self):
        white = np.full((48, 48), 255, dtype=np.uint8)
        assert not apply_newsprint_dither(white).any()

    def test_black_is_fully_engraved(self):
        black = np.zeros((48, 48), dtype=np.uint8)
        assert apply_newsprint_dither(black).all()

    def test_coverage_follows_darkness(self):
        grayscale = np.full((64, 64), 128, dtype=np.uint8)
        result = apply_newsprint_dither(grayscale)
        assert result.mean() == pytest.approx(1.0 - 128 / 255.0, abs=0.03)

    def test_dots_are_clustered(self):
        """A clustered-dot screen forms far fewer separate dots than a
        dispersed (Bayer) screen at the same tone."""
        grayscale = np.full((64, 64), 200, dtype=np.uint8)
        news = apply_newsprint_dither(grayscale)
        bayer = apply_bayer_dither(grayscale, BAYER8, False)
        assert len(dot_boxes(news)) < len(dot_boxes(bayer)) / 2

    def test_cell_follows_cell_size(self):
        grayscale = np.full((64, 64), 200, dtype=np.uint8)
        fine = apply_newsprint_dither(grayscale, 1)
        coarse = apply_newsprint_dither(grayscale, 2)
        assert len(dot_boxes(coarse)) < len(dot_boxes(fine))

    def test_invert_is_the_complement(self):
        rng = np.random.default_rng(17)
        grayscale = rng.integers(0, 256, (48, 48)).astype(np.uint8)
        inverted = apply_newsprint_dither(grayscale, invert=True)
        plain = apply_newsprint_dither(grayscale)
        np.testing.assert_array_equal(inverted, 1 - plain)

    def test_non_positive_pixels_per_mm_raises(self):
        grayscale = np.zeros((4, 4), dtype=np.uint8)
        with pytest.raises(ValueError, match="pixels_per_mm"):
            apply_newsprint_dither(grayscale, pixels_per_mm=(1.0, 0.0))

import numpy as np

from raygeo.image.pack import Packer


def _as_u8(mask: np.ndarray) -> np.ndarray:
    return np.ascontiguousarray(mask, dtype=np.uint8)


def _reference_first_fit(
    canvas: np.ndarray, mask: np.ndarray
) -> tuple[int, int] | None:
    height, width = canvas.shape
    mask_height, mask_width = mask.shape
    if mask_height > height or mask_width > width:
        return None
    for y in range(height - mask_height + 1):
        for x in range(width - mask_width + 1):
            window = canvas[y : y + mask_height, x : x + mask_width]
            if not np.any(window & mask):
                return y, x
    return None


def _reference_score(
    bounds: list[tuple[int, int, int, int]],
    x: int,
    y: int,
    width: int,
    height: int,
) -> int:
    min_x = min([x] + [bound[0] for bound in bounds])
    min_y = min([y] + [bound[1] for bound in bounds])
    max_x = max([x + width] + [bound[2] for bound in bounds])
    max_y = max([y + height] + [bound[3] for bound in bounds])
    return (max_x - min_x) * (max_y - min_y)


class _ReferencePacker:
    """Straightforward implementation used to cross-check the Rust one."""

    def __init__(self, canvas: np.ndarray):
        self.canvas = canvas.astype(bool)
        self.bounds: list[tuple[int, int, int, int]] = []

    def place(self, masks: list[np.ndarray]) -> tuple[int, int, int] | None:
        best = None
        best_score = None
        for index, mask in enumerate(masks):
            position = _reference_first_fit(self.canvas, mask)
            if position is None:
                continue
            y, x = position
            score = _reference_score(
                self.bounds, x, y, mask.shape[1], mask.shape[0]
            )
            if best_score is None or score < best_score:
                best_score = score
                best = (index, y, x)

        if best is None:
            return None

        index, y, x = best
        mask = masks[index]
        self.canvas[y : y + mask.shape[0], x : x + mask.shape[1]] |= mask
        self.bounds.append((x, y, x + mask.shape[1], y + mask.shape[0]))
        return best


def test_empty_canvas_places_at_origin():
    packer = Packer(np.zeros((6, 8), dtype=np.uint8))
    mask = np.ones((2, 3), dtype=np.uint8)

    assert packer.place([mask]) == (0, 0, 0)


def test_first_fit_scans_row_major():
    canvas = np.zeros((6, 8), dtype=np.uint8)
    canvas[0, 0:2] = 1
    packer = Packer(canvas)
    mask = np.ones((2, 2), dtype=np.uint8)

    assert packer.place([mask]) == (0, 0, 2)


def test_occupied_row_pushes_placement_down():
    canvas = np.zeros((6, 8), dtype=np.uint8)
    canvas[0, :] = 1
    packer = Packer(canvas)
    mask = np.ones((1, 1), dtype=np.uint8)

    assert packer.place([mask]) == (0, 1, 0)


def test_chooses_variant_with_smallest_bounding_box():
    packer = Packer(np.zeros((10, 10), dtype=np.uint8))
    large = np.ones((2, 2), dtype=np.uint8)
    small = np.ones((1, 1), dtype=np.uint8)

    assert packer.place([large, small]) == (1, 0, 0)


def test_tie_prefers_earlier_variant():
    packer = Packer(np.zeros((10, 10), dtype=np.uint8))
    first = np.ones((2, 2), dtype=np.uint8)
    second = np.ones((2, 2), dtype=np.uint8)

    assert packer.place([first, second]) == (0, 0, 0)


def test_oversized_variant_is_skipped():
    packer = Packer(np.zeros((6, 6), dtype=np.uint8))
    too_large = np.ones((7, 7), dtype=np.uint8)
    fits = np.ones((1, 1), dtype=np.uint8)

    assert packer.place([too_large, fits]) == (1, 0, 0)


def test_zero_size_variant_is_skipped():
    packer = Packer(np.zeros((6, 6), dtype=np.uint8))
    empty = np.zeros((0, 0), dtype=np.uint8)
    fits = np.ones((1, 1), dtype=np.uint8)

    assert packer.place([empty, fits]) == (1, 0, 0)


def test_returns_none_when_nothing_fits():
    canvas = np.ones((4, 4), dtype=np.uint8)
    packer = Packer(canvas)

    assert packer.place([np.ones((1, 1), dtype=np.uint8)]) is None


def test_placement_marks_the_canvas():
    packer = Packer(np.zeros((6, 6), dtype=np.uint8))
    first = np.ones((2, 2), dtype=np.uint8)
    second = np.ones((1, 1), dtype=np.uint8)

    assert packer.place([first]) == (0, 0, 0)
    assert packer.place([second]) == (0, 0, 2)


def test_nonzero_mask_values_are_foreground():
    packer = Packer(np.zeros((5, 5), dtype=np.uint8))
    mask = np.full((2, 2), 255, dtype=np.uint8)

    assert packer.place([mask]) == (0, 0, 0)


def test_matches_reference_on_random_layouts():
    rng = np.random.default_rng(1234)
    for _ in range(30):
        height = int(rng.integers(1, 14))
        width = int(rng.integers(1, 14))
        canvas = rng.random((height, width)) < 0.15

        packer = Packer(_as_u8(canvas))
        reference = _ReferencePacker(canvas)

        for _ in range(6):
            masks = []
            for _ in range(int(rng.integers(1, 4))):
                mask_height = int(rng.integers(1, min(height, 6) + 1))
                mask_width = int(rng.integers(1, min(width, 6) + 1))
                mask = rng.random((mask_height, mask_width)) < 0.4
                if not mask.any():
                    mask[0, 0] = True
                masks.append(mask)

            expected = reference.place(masks)
            result = packer.place([_as_u8(mask) for mask in masks])
            assert result == expected

"""Tests for ``raygeo.ops.part.image_source.WholeImageSource`` and the
``image_source`` property on ``raygeo.ops.part.Part``.
"""

import numpy as np
import pytest

from raygeo.ops.assembly.raster import raster
from raygeo.ops.assembly.shrinkwrap import shrinkwrap
from raygeo.ops.part import Part
from raygeo.ops.part.image_source import WholeImageSource

# ---------------------------------------------------------------------------
# WholeImageSource
# ---------------------------------------------------------------------------


def _checkerboard(w: int, h: int) -> np.ndarray:
    """A small uint8 array whose row i, col j = (i + j) % 256.

    Useful for verifying row-major order in slab reads.
    """
    arr = np.fromfunction(
        lambda y, x: (y + x).astype(np.uint8), (h, w), dtype=np.uint64
    )
    return arr.astype(np.uint8)


def test_construct_from_2d_uint8_array():
    arr = np.full((4, 7), 128, dtype=np.uint8)
    src = WholeImageSource(arr)
    assert src.dimensions == (7, 4)
    assert src.width == 7
    assert src.height == 4


def test_construct_upcasts_non_uint8_array():
    """A float array is accepted; the result is uint8-clamped."""
    arr = np.zeros((2, 3), dtype=np.float32)
    arr[0, :] = 1.0  # clamps to 1
    arr[1, :] = 254.9  # clamps to 254
    src = WholeImageSource(arr)
    buf = src.read_all()
    assert buf is not None
    # First row (clamped to 1), second row (clamped to 254).
    assert list(buf[:3]) == [1, 1, 1]
    assert list(buf[3:6]) == [254, 254, 254]


def test_construct_rejects_zero_dimension():
    with pytest.raises(ValueError, match="zero dimension"):
        WholeImageSource(np.zeros((0, 5), dtype=np.uint8))
    with pytest.raises(ValueError, match="zero dimension"):
        WholeImageSource(np.zeros((4, 0), dtype=np.uint8))


def test_construct_rejects_1d_array():
    """The setter expects a 2-D array (height, width)."""
    with pytest.raises((ValueError, TypeError)):
        WholeImageSource(np.array([1, 2, 3], dtype=np.uint8))


def test_read_all_returns_full_row_major_buffer():
    arr = _checkerboard(3, 4)
    src = WholeImageSource(arr)
    buf = src.read_all()
    assert buf is not None
    assert len(buf) == 12
    expected = arr.flatten().tolist()
    assert list(buf) == expected


def test_read_all_is_bytes_like():
    """``read_all`` returns PyO3's ``bytes`` representation of Vec<u8>."""
    src = WholeImageSource(np.full((2, 3), 100, dtype=np.uint8))
    buf = src.read_all()
    assert buf is not None
    assert isinstance(buf, (bytes, bytearray))


def test_read_slab_returns_requested_rows():
    arr = _checkerboard(5, 6)
    src = WholeImageSource(arr)
    # Rows 1..3 (i.e. indices 1 and 2).
    slab = src.read_slab(1, 3)
    assert len(slab) == 2 * 5
    expected = arr[1:3].flatten().tolist()
    assert list(slab) == expected


def test_read_slab_clamps_to_bottom_edge():
    arr = _checkerboard(3, 4)
    src = WholeImageSource(arr)
    # Request beyond the bottom: only rows 2..4 are available.
    slab = src.read_slab(2, 10)
    assert len(slab) == 2 * 3
    expected = arr[2:4].flatten().tolist()
    assert list(slab) == expected


def test_read_slab_empty_when_start_at_bottom():
    src = WholeImageSource(np.full((3, 2), 1, dtype=np.uint8))
    assert src.read_slab(3, 5) == b""


def test_read_slab_empty_when_inverted_range():
    src = WholeImageSource(np.full((3, 2), 1, dtype=np.uint8))
    assert src.read_slab(2, 1) == b""


def test_read_slab_empty_when_both_indices_zero():
    src = WholeImageSource(np.full((3, 2), 1, dtype=np.uint8))
    assert src.read_slab(0, 0) == b""


def test_is_cancelled_is_false():
    src = WholeImageSource(np.zeros((2, 2), dtype=np.uint8))
    assert src.is_cancelled() is False


def test_repr_includes_dimensions():
    src = WholeImageSource(np.zeros((4, 7), dtype=np.uint8))
    assert repr(src) == "WholeImageSource(width=7, height=4)"


# ---------------------------------------------------------------------------
# Part.image_source property
# ---------------------------------------------------------------------------


def test_part_image_source_starts_none():
    part = Part(size_mm=(10.0, 10.0))
    assert part.image_source is None
    assert part.image is None


def test_part_image_source_setter_accepts_whole_image_source():
    arr = np.full((20, 15), 200, dtype=np.uint8)
    src = WholeImageSource(arr)
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(2.0, 2.0))
    part.image_source = src
    # Identity-preserving getter.
    assert part.image_source is src
    assert part.image_source.dimensions == (15, 20)
    # Legacy `image` reads through the same source.
    assert part.image is not None
    assert len(part.image) == 20 * 15


def test_part_image_source_setter_none_clears():
    arr = np.full((5, 5), 255, dtype=np.uint8)
    src = WholeImageSource(arr)
    part = Part(size_mm=(10.0, 10.0))
    part.image_source = src
    assert part.image_source is not None
    part.image_source = None
    assert part.image_source is None
    assert part.image is None


def test_part_image_setter_constructs_image_source_internally():
    """The numpy `image` setter builds a WholeImageSource internally."""
    arr = np.full((10, 8), 100, dtype=np.uint8)
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(1.0, 1.0))
    part.image = arr
    src = part.image_source
    assert isinstance(src, WholeImageSource)
    assert src.dimensions == (8, 10)
    # Setting image with None also clears image_source.
    part.image = None
    assert part.image_source is None
    assert part.image is None


def test_part_image_setter_rejects_zero_dimension():
    part = Part(size_mm=(10.0, 10.0))
    with pytest.raises(ValueError, match="zero dimension"):
        part.image = np.zeros((0, 5), dtype=np.uint8)
    # The part should remain in a clean state with no source attached.
    assert part.image_source is None


def test_part_image_and_image_source_share_storage():
    """Assigning via `image` exposes the source through `image_source`,
    and assigning via `image_source` exposes the buffer through `image`.
    """
    arr = np.full((6, 9), 50, dtype=np.uint8)
    part = Part(size_mm=(10.0, 10.0))
    part.image = arr
    src = part.image_source
    assert src is not None
    assert src.dimensions == (9, 6)
    assert part.image == src.read_all()

    # Now replace via image_source.
    arr2 = np.full((6, 9), 200, dtype=np.uint8)
    src2 = WholeImageSource(arr2)
    part.image_source = src2
    assert part.image == src2.read_all()
    assert part.image_source is src2


# ---------------------------------------------------------------------------
# End-to-end: raster and shrinkwrap read through the new source
# ---------------------------------------------------------------------------


def _filled_part(fill: int = 255) -> Part:
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0))
    arr = np.full((100, 100), fill, dtype=np.uint8)
    part.image = arr
    return part


def test_raster_reads_through_image_source():
    """The raster assembler reads pixels through ``Part.image_source``."""
    part = _filled_part()
    result = raster(part, angle=0.0, mode="mask_scan", scan_mode="segmented")
    assert len(result.ops) > 0


def test_shrinkwrap_reads_through_image_source():
    """The shrinkwrap assembler reads pixels through
    ``Part.image_source``.
    """
    part = _filled_part()
    result = shrinkwrap(
        part,
        gravity=0.0,
        kerf_mm=0.0,
        path_offset_mm=0.0,
        cut_side="outer",
        arc_tolerance=0.0,
        allow_arcs=False,
        supports_curves=False,
    )
    assert len(result.ops) > 0


def test_raster_works_when_image_attached_via_image_source_setter():
    """Same as test_raster_reads_through_image_source but using the
    typed `image_source` setter rather than the numpy `image` shim.
    """
    arr = np.full((100, 100), 255, dtype=np.uint8)
    src = WholeImageSource(arr)
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0))
    part.image_source = src
    result = raster(part, angle=0.0, mode="mask_scan", scan_mode="segmented")
    assert len(result.ops) > 0


def test_raster_fails_when_no_image_source():
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0))
    with pytest.raises(ValueError, match="Part has no image"):
        raster(part, angle=0.0, mode="mask_scan", scan_mode="segmented")


def test_shrinkwrap_fails_when_no_image_source():
    part = Part(size_mm=(10.0, 10.0))
    with pytest.raises(ValueError, match="Part has no image"):
        shrinkwrap(
            part,
            gravity=0.0,
            kerf_mm=0.0,
            path_offset_mm=0.0,
            cut_side="outer",
            arc_tolerance=0.0,
            allow_arcs=False,
            supports_curves=False,
        )

"""Tests for ``raygeo.ops.part.image_source.VipsChunkSource`` and its
integration with ``Part`` and the raster assembler.

Uses a lightweight duck-typed stand-in for ``pyvips.Image`` so tests
run without the libvips system dependency.
"""

import numpy as np
import pytest

from raygeo.ops.assembly.raster import raster
from raygeo.ops.assembly.shrinkwrap import shrinkwrap
from raygeo.ops.part import Part
from raygeo.ops.part.image_source import VipsChunkSource, WholeImageSource


class FakeVipsImage:
    """Minimal duck-typed stand-in for ``pyvips.Image``.

    Exposes ``width``, ``height``, ``bands``, ``crop``, and
    ``write_to_memory`` — the exact surface ``VipsChunkSource`` calls
    via the GIL.
    """

    def __init__(self, data: np.ndarray, bands: int = 1):
        if data.ndim != 2:
            raise ValueError(f"Expected 2-D array, got {data.ndim}-D")
        self._data = data
        self.width = data.shape[1]
        self.height = data.shape[0]
        self.bands = bands

    def crop(self, left, top, width, height):
        return FakeVipsImage(
            self._data[top : top + height, left : left + width],
            bands=self.bands,
        )

    def write_to_memory(self):
        return self._data.tobytes()


def _checkerboard(w, h):
    arr = np.fromfunction(
        lambda y, x: (y + x).astype(np.uint8), (h, w), dtype=np.uint64
    )
    return arr.astype(np.uint8)


# ---------------------------------------------------------------------------
# Construction
# ---------------------------------------------------------------------------


def test_construct_from_fake_vips_image():
    img = FakeVipsImage(np.full((4, 7), 128, dtype=np.uint8))
    src = VipsChunkSource(img)
    assert src.dimensions == (7, 4)
    assert src.width == 7
    assert src.height == 4


def test_construct_rejects_zero_dimension():
    with pytest.raises(ValueError, match="zero dimension"):
        VipsChunkSource(FakeVipsImage(np.zeros((0, 5), dtype=np.uint8)))


def test_construct_rejects_multiband():
    img = FakeVipsImage(np.full((4, 7), 128, dtype=np.uint8), bands=3)
    with pytest.raises(ValueError, match="single-band"):
        VipsChunkSource(img)


def test_repr_includes_dimensions():
    img = FakeVipsImage(np.zeros((4, 7), dtype=np.uint8))
    src = VipsChunkSource(img)
    assert repr(src) == "VipsChunkSource(width=7, height=4)"


def test_default_threshold_is_256_mb():
    """Default in_memory_threshold_mb is 256 (268 435 456 bytes)."""
    img = FakeVipsImage(np.zeros((10, 10), dtype=np.uint8))
    src = VipsChunkSource(img)
    assert src.dimensions == (10, 10)
    # 10*10 = 100 bytes << 256 MB → read_all succeeds.
    assert src.read_all() is not None


# ---------------------------------------------------------------------------
# read_slab
# ---------------------------------------------------------------------------


def test_read_slab_returns_requested_rows():
    arr = _checkerboard(5, 6)
    src = VipsChunkSource(FakeVipsImage(arr))
    slab = src.read_slab(1, 3)
    assert len(slab) == 2 * 5
    expected = arr[1:3].flatten().tolist()
    assert list(slab) == expected


def test_read_slab_clamps_to_bottom_edge():
    arr = _checkerboard(3, 4)
    src = VipsChunkSource(FakeVipsImage(arr))
    slab = src.read_slab(2, 10)
    assert len(slab) == 2 * 3
    expected = arr[2:4].flatten().tolist()
    assert list(slab) == expected


def test_read_slab_empty_when_start_at_bottom():
    src = VipsChunkSource(FakeVipsImage(np.zeros((3, 2), dtype=np.uint8)))
    assert src.read_slab(3, 5) == b""


def test_read_slab_single_row():
    arr = _checkerboard(4, 3)
    src = VipsChunkSource(FakeVipsImage(arr))
    slab = src.read_slab(1, 2)
    assert len(slab) == 4
    assert list(slab) == arr[1].tolist()


def test_read_slab_data_matches_whole_image_source():
    """Slab reads from VipsChunkSource and WholeImageSource produce
    identical bytes for the same underlying data.
    """
    arr = _checkerboard(8, 8)
    whole = WholeImageSource(arr)
    vips = VipsChunkSource(FakeVipsImage(arr))

    for y_start in range(0, 8, 2):
        assert list(whole.read_slab(y_start, y_start + 2)) == list(
            vips.read_slab(y_start, y_start + 2)
        )


# ---------------------------------------------------------------------------
# read_all
# ---------------------------------------------------------------------------


def test_read_all_returns_full_buffer_for_small_image():
    arr = _checkerboard(5, 4)
    src = VipsChunkSource(FakeVipsImage(arr))
    buf = src.read_all()
    assert buf is not None
    assert len(buf) == 20
    assert list(buf) == arr.flatten().tolist()


def test_read_all_returns_none_above_threshold():
    """When the image exceeds the threshold, read_all returns None."""
    arr = np.zeros((100, 100), dtype=np.uint8)
    img = FakeVipsImage(arr)
    # Threshold = 0 MB → even 1 byte is too much.
    src = VipsChunkSource(img, in_memory_threshold_mb=0)
    assert src.read_all() is None


def test_read_all_threshold_boundary():
    """A 100-byte image is well under the 1 MB threshold."""
    w = 10
    h = 10
    src = VipsChunkSource(
        FakeVipsImage(np.zeros((h, w), dtype=np.uint8)),
        in_memory_threshold_mb=1,
    )
    assert src.read_all() is not None


def test_is_cancelled_is_false():
    src = VipsChunkSource(FakeVipsImage(np.zeros((2, 2), dtype=np.uint8)))
    assert src.is_cancelled() is False


# ---------------------------------------------------------------------------
# Part.image_source integration
# ---------------------------------------------------------------------------


def test_part_accepts_vips_chunk_source():
    img = FakeVipsImage(np.full((20, 15), 200, dtype=np.uint8))
    src = VipsChunkSource(img)
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(2.0, 2.0))
    part.image_source = src
    assert part.image_source is src
    assert part.image_source.dimensions == (15, 20)


def test_part_accepts_both_source_types():
    """Switching between WholeImageSource and VipsChunkSource works."""
    arr = np.full((10, 10), 255, dtype=np.uint8)
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(1.0, 1.0))

    whole = WholeImageSource(arr)
    part.image_source = whole
    assert isinstance(part.image_source, WholeImageSource)

    vips = VipsChunkSource(FakeVipsImage(arr))
    part.image_source = vips
    assert isinstance(part.image_source, VipsChunkSource)

    part.image_source = None
    assert part.image_source is None


def test_part_image_source_setter_rejects_wrong_type():
    part = Part(size_mm=(10.0, 10.0))
    with pytest.raises(TypeError, match="WholeImageSource or VipsChunkSource"):
        part.image_source = 42


def test_part_image_returns_none_when_vips_above_threshold():
    """When VipsChunkSource.read_all() returns None, part.image is None."""
    src = VipsChunkSource(
        FakeVipsImage(np.zeros((100, 100), dtype=np.uint8)),
        in_memory_threshold_mb=0,
    )
    part = Part(size_mm=(10.0, 10.0))
    part.image_source = src
    assert part.image is None


# ---------------------------------------------------------------------------
# Raster assembler
# ---------------------------------------------------------------------------


def _filled_part_vips(fill=255, threshold_mb=256):
    arr = np.full((100, 100), fill, dtype=np.uint8)
    src = VipsChunkSource(
        FakeVipsImage(arr), in_memory_threshold_mb=threshold_mb
    )
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0))
    part.image_source = src
    return part


def test_raster_works_with_vips_source_read_all_path():
    """Raster works when VipsChunkSource.read_all() succeeds."""
    part = _filled_part_vips(threshold_mb=256)
    result = raster(part, angle=0.0, mode="mask_scan", scan_mode="segmented")
    assert len(result.ops) > 0


def test_raster_works_with_vips_source_slab_path():
    """Raster falls back to slab-by-slab when read_all() returns None."""
    part = _filled_part_vips(threshold_mb=0)
    result = raster(part, angle=0.0, mode="mask_scan", scan_mode="segmented")
    assert len(result.ops) > 0


def test_raster_slab_path_matches_read_all_path():
    """Output is identical regardless of whether read_all() succeeds."""
    arr = np.full((100, 100), 255, dtype=np.uint8)
    img = FakeVipsImage(arr)

    part_a = Part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0))
    part_a.image_source = VipsChunkSource(img, in_memory_threshold_mb=256)
    result_a = raster(
        part_a, angle=0.0, mode="mask_scan", scan_mode="segmented"
    )

    part_b = Part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0))
    part_b.image_source = VipsChunkSource(img, in_memory_threshold_mb=0)
    result_b = raster(
        part_b, angle=0.0, mode="mask_scan", scan_mode="segmented"
    )

    assert len(result_a.ops) == len(result_b.ops)
    assert result_a.ops.to_dict() == result_b.ops.to_dict()


# ---------------------------------------------------------------------------
# Shrinkwrap degradation
# ---------------------------------------------------------------------------


def test_shrinkwrap_fails_when_read_all_returns_none():
    """Shrinkwrap requires read_all(); it degrades when None."""
    part = _filled_part_vips(threshold_mb=0)
    with pytest.raises(ValueError, match="cannot materialise a full buffer"):
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


def test_shrinkwrap_works_when_read_all_succeeds():
    part = _filled_part_vips(threshold_mb=256)
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

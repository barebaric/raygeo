import cairo
import numpy as np

from raygeo.image.convert import (
    rgba_to_binary,
    rgba_to_grayscale,
    rgba_to_grayscale_inplace,
)


def _surface_to_flat(surface):
    width = surface.get_width()
    height = surface.get_height()
    stride = surface.get_stride() // 4
    buf = surface.get_data()
    # Cairo ARGB32 stores native-endian 0xAARRGGBB pixels; the conversion
    # functions accept exactly this layout, so no byte-order translation
    # happens here (matching how rayforge feeds surface data in).
    data_with_padding = np.ndarray(
        shape=(height, stride, 4), dtype=np.uint8, buffer=buf
    )
    return data_with_padding.flatten(), width, height, stride


def _fill_pixels(surface, a, r, g, b):
    """Overwrite every pixel with the given ARGB32 components.

    The pixel is assembled as a 32-bit 0xAARRGGBB value and written through
    a native-endian view, so the surface content is byte-order independent.
    """
    height = surface.get_height()
    stride_px = surface.get_stride() // 4
    pixels = np.ndarray(
        shape=(height, stride_px), dtype=np.uint32, buffer=surface.get_data()
    )
    pixels[:, : surface.get_width()] = (a << 24) | (r << 16) | (g << 8) | b
    surface.mark_dirty()


def _flat_to_bgra(flat, height, stride):
    """Decode a flat ARGB32 buffer into per-pixel (B, G, R, A) channels.

    Cairo ARGB32 pixels are native-endian 0xAARRGGBB values, so decoding
    via bit shifts is byte-order independent.
    """
    pixels = np.asarray(flat).view(np.uint32).reshape(height, stride)
    bgra = np.empty((height, stride, 4), dtype=np.uint8)
    bgra[:, :, 0] = pixels & 0xFF
    bgra[:, :, 1] = (pixels >> 8) & 0xFF
    bgra[:, :, 2] = (pixels >> 16) & 0xFF
    bgra[:, :, 3] = (pixels >> 24) & 0xFF
    return bgra


# ---------------------------------------------------------------------------
# rgba_to_grayscale
# ---------------------------------------------------------------------------


class TestRgbaToGrayscale:
    def test_black_surface(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 10)
        ctx = cairo.Context(surface)
        ctx.set_source_rgb(0, 0, 0)
        ctx.paint()
        flat, w, h, stride = _surface_to_flat(surface)
        gray, alpha = rgba_to_grayscale(flat, w, h, stride)
        assert gray.shape == (10, 10)
        assert alpha.shape == (10, 10)
        assert np.all(gray == 0)
        assert np.allclose(alpha, 1.0)

    def test_white_surface(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 10)
        ctx = cairo.Context(surface)
        ctx.set_source_rgb(1, 1, 1)
        ctx.paint()
        flat, w, h, stride = _surface_to_flat(surface)
        gray, alpha = rgba_to_grayscale(flat, w, h, stride)
        assert np.allclose(gray, 255, atol=1)
        assert np.all(alpha == 1.0)

    def test_transparent_surface(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 10)
        ctx = cairo.Context(surface)
        ctx.set_source_rgba(0.5, 0.5, 0.5, 0)
        ctx.paint()
        flat, w, h, stride = _surface_to_flat(surface)
        gray, alpha = rgba_to_grayscale(flat, w, h, stride)
        assert np.all(alpha == 0.0)


# ---------------------------------------------------------------------------
# rgba_to_binary
# ---------------------------------------------------------------------------


class TestRgbaToBinary:
    def test_black_surface_becomes_all_ones(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 10)
        ctx = cairo.Context(surface)
        ctx.set_source_rgb(0, 0, 0)
        ctx.paint()
        flat, w, h, stride = _surface_to_flat(surface)
        binary = rgba_to_binary(flat, w, h, stride, threshold=128)
        assert binary.shape == (10, 10)
        assert np.all(binary == 1)

    def test_white_surface_becomes_all_zeros(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 10)
        ctx = cairo.Context(surface)
        ctx.set_source_rgb(1, 1, 1)
        ctx.paint()
        flat, w, h, stride = _surface_to_flat(surface)
        binary = rgba_to_binary(flat, w, h, stride, threshold=128)
        assert np.all(binary == 0)

    def test_threshold_behavior(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=50, g=50, b=50)
        flat, w, h, stride = _surface_to_flat(surface)
        binary_low = rgba_to_binary(flat, w, h, stride, threshold=40)
        assert np.all(binary_low == 0)
        binary_high = rgba_to_binary(flat, w, h, stride, threshold=60)
        assert np.all(binary_high == 1)

    def test_invert_mode(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 10)
        ctx = cairo.Context(surface)
        ctx.set_source_rgb(1, 1, 1)
        ctx.paint()
        flat, w, h, stride = _surface_to_flat(surface)
        binary = rgba_to_binary(flat, w, h, stride, threshold=128, invert=True)
        assert np.all(binary == 1)

    def test_transparent_becomes_zero(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 10)
        ctx = cairo.Context(surface)
        ctx.set_source_rgba(0, 0, 0, 0)
        ctx.paint()
        flat, w, h, stride = _surface_to_flat(surface)
        binary = rgba_to_binary(flat, w, h, stride, threshold=128)
        assert np.all(binary == 0)

    def test_partial_opacity_preserved(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=128, r=0, g=0, b=0)
        flat, w, h, stride = _surface_to_flat(surface)
        binary = rgba_to_binary(flat, w, h, stride, threshold=128)
        assert np.all(binary == 1)

    def test_output_is_binary(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 10)
        ctx = cairo.Context(surface)
        gradient = cairo.LinearGradient(0, 0, 10, 10)
        gradient.add_color_stop_rgb(0, 0, 0, 0)
        gradient.add_color_stop_rgb(1, 1, 1, 1)
        ctx.set_source(gradient)
        ctx.paint()
        flat, w, h, stride = _surface_to_flat(surface)
        binary = rgba_to_binary(flat, w, h, stride, threshold=128)
        unique_values = np.unique(binary)
        assert all(v in [0, 1] for v in unique_values)


# ---------------------------------------------------------------------------
# rgba_to_grayscale_inplace
# ---------------------------------------------------------------------------


class TestRgbaToGrayscaleInplace:
    def test_converts_to_grayscale(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=0, g=0, b=255)
        flat, w, h, stride = _surface_to_flat(surface)
        rgba_to_grayscale_inplace(flat, w, h, stride)
        bgra = _flat_to_bgra(flat, h, stride)
        assert bgra[0, 0, 0] == bgra[0, 0, 1]
        assert bgra[0, 0, 1] == bgra[0, 0, 2]

    def test_preserves_alpha(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=128, r=200, g=150, b=100)
        flat, w, h, stride = _surface_to_flat(surface)
        rgba_to_grayscale_inplace(flat, w, h, stride)
        bgra = _flat_to_bgra(flat, h, stride)
        assert bgra[0, 0, 3] == 128

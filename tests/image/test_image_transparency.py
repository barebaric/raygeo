import cairo
import numpy as np

from raygeo.image.transparency import (
    make_transparent_by_brightness,
    make_transparent_except_color,
)


def _surface_to_flat(surface):
    width = surface.get_width()
    height = surface.get_height()
    stride_px = surface.get_stride() // 4
    buf = surface.get_data()
    data = np.frombuffer(buf, dtype=np.uint8).copy()
    return data, width, height, stride_px


def _fill_pixels(surface, a, r, g, b):
    """Overwrite every pixel with the given ARGB32 components.

    The pixel is assembled as a 32-bit 0xAARRGGBB value and written
    through a native-endian view, so the surface content does not
    depend on the host byte order.
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
# make_transparent_by_brightness
# ---------------------------------------------------------------------------


class TestMakeTransparentByBrightness:
    def test_white_pixels_become_transparent(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=255, g=255, b=255)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=250)

        bgra = _flat_to_bgra(flat, h, stride)
        assert np.all(bgra[:w, :, 3] == 0)

    def test_dark_pixels_stay_opaque(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=0, g=0, b=0)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=250)

        bgra = _flat_to_bgra(flat, h, stride)
        assert np.all(bgra[:w, :, 3] == 255)

    def test_custom_threshold(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=200, g=200, b=200)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=150)

        bgra = _flat_to_bgra(flat, h, stride)
        assert np.all(bgra[:w, :, 3] == 0)

    def test_threshold_boundary(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=200, g=200, b=200)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=201)

        bgra = _flat_to_bgra(flat, h, stride)
        brightness = (77 * 200 + 150 * 200 + 29 * 200) >> 8
        assert brightness == 200
        assert np.all(bgra[:w, :, 3] == 255)

    def test_default_threshold(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=255, g=255, b=255)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride)

        bgra = _flat_to_bgra(flat, h, stride)
        assert np.all(bgra[:w, :, 3] == 0)

    def test_preserves_color_channels(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=200, g=150, b=100)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=10)

        bgra = _flat_to_bgra(flat, h, stride)
        assert bgra[0, 0, 0] == 100
        assert bgra[0, 0, 1] == 150
        assert bgra[0, 0, 2] == 200
        assert bgra[0, 0, 3] == 0

    def test_weighted_brightness_not_average(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 1, 1)
        _fill_pixels(surface, a=255, r=0, g=0, b=255)

        flat, w, h, stride = _surface_to_flat(surface)
        brightness = (77 * 0 + 150 * 0 + 29 * 255) >> 8
        assert brightness == 28

        make_transparent_by_brightness(flat, w, h, stride, threshold=29)
        bgra = _flat_to_bgra(flat, h, stride)
        assert bgra[0, 0, 3] == 255

        make_transparent_by_brightness(flat, w, h, stride, threshold=28)
        bgra = _flat_to_bgra(flat, h, stride)
        assert bgra[0, 0, 3] == 0


# ---------------------------------------------------------------------------
# make_transparent_except_color
# ---------------------------------------------------------------------------


class TestMakeTransparentExceptColor:
    def test_target_color_stays_opaque(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=200, g=150, b=100)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_except_color(
            flat, w, h, stride, target_r=200, target_g=150, target_b=100
        )

        bgra = _flat_to_bgra(flat, h, stride)
        assert np.all(bgra[:w, :, 3] == 255)

    def test_non_matching_becomes_transparent(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=150, g=100, b=50)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_except_color(
            flat, w, h, stride, target_r=200, target_g=150, target_b=100
        )

        bgra = _flat_to_bgra(flat, h, stride)
        assert np.all(bgra[:w, :, 3] == 0)

    def test_mixed_colors(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        stride_px = surface.get_stride() // 4
        pixels = np.ndarray(
            shape=(2, stride_px), dtype=np.uint32, buffer=surface.get_data()
        )
        green = (255 << 24) | (0 << 16) | (255 << 8) | 0
        blue = (255 << 24) | (0 << 16) | (0 << 8) | 255
        red = (255 << 24) | (255 << 16) | (0 << 8) | 0
        pixels[0, 0] = green
        pixels[0, 1] = blue
        pixels[1, 0] = green
        pixels[1, 1] = red
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_except_color(
            flat, w, h, stride, target_r=0, target_g=255, target_b=0
        )

        bgra = _flat_to_bgra(flat, h, stride)
        assert bgra[0, 0, 3] == 255
        assert bgra[0, 1, 3] == 0
        assert bgra[1, 0, 3] == 255
        assert bgra[1, 1, 3] == 0

    def test_preserves_color_channels(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        _fill_pixels(surface, a=255, r=200, g=100, b=50)

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_except_color(
            flat, w, h, stride, target_r=255, target_g=0, target_b=0
        )

        bgra = _flat_to_bgra(flat, h, stride)
        assert bgra[0, 0, 0] == 50
        assert bgra[0, 0, 1] == 100
        assert bgra[0, 0, 2] == 200
        assert bgra[0, 0, 3] == 0

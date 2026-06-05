import cairo
import numpy as np
from raygeo.image import (
    rgba_to_binary,
    rgba_to_grayscale,
    rgba_to_grayscale_inplace,
)


def _surface_to_flat(surface):
    width = surface.get_width()
    height = surface.get_height()
    stride = surface.get_stride() // 4
    buf = surface.get_data()
    data_with_padding = np.ndarray(
        shape=(height, stride, 4), dtype=np.uint8, buffer=buf
    )
    return data_with_padding.flatten(), width, height, stride


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
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 50
        arr[:, :, 1] = 50
        arr[:, :, 2] = 50
        arr[:, :, 3] = 255
        surface.mark_dirty()
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
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 0
        arr[:, :, 1] = 0
        arr[:, :, 2] = 0
        arr[:, :, 3] = 128
        surface.mark_dirty()
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
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 255
        arr[:, :, 1] = 0
        arr[:, :, 2] = 0
        arr[:, :, 3] = 255
        surface.mark_dirty()
        flat, w, h, stride = _surface_to_flat(surface)
        rgba_to_grayscale_inplace(flat, w, h, stride)
        arr_out = flat.reshape(h, stride, 4)
        assert arr_out[0, 0, 0] == arr_out[0, 0, 1]
        assert arr_out[0, 0, 1] == arr_out[0, 0, 2]

    def test_preserves_alpha(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 100
        arr[:, :, 1] = 150
        arr[:, :, 2] = 200
        arr[:, :, 3] = 128
        surface.mark_dirty()
        flat, w, h, stride = _surface_to_flat(surface)
        rgba_to_grayscale_inplace(flat, w, h, stride)
        arr_out = flat.reshape(h, stride, 4)
        assert arr_out[0, 0, 3] == 128

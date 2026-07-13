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


def _read_alpha(surface):
    data = surface.get_data()
    stride = surface.get_stride()
    arr = np.frombuffer(data, dtype=np.uint8).reshape(
        (surface.get_height(), stride, 4)
    )
    return arr[:, : surface.get_width(), 3]


# ---------------------------------------------------------------------------
# make_transparent_by_brightness
# ---------------------------------------------------------------------------


class TestMakeTransparentByBrightness:
    def test_white_pixels_become_transparent(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 255
        arr[:, :, 1] = 255
        arr[:, :, 2] = 255
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=250)

        flat_out = flat.reshape(h, stride, 4)
        assert np.all(flat_out[:w, :, 3] == 0)

    def test_dark_pixels_stay_opaque(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 0
        arr[:, :, 1] = 0
        arr[:, :, 2] = 0
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=250)

        flat_out = flat.reshape(h, stride, 4)
        assert np.all(flat_out[:w, :, 3] == 255)

    def test_custom_threshold(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 200
        arr[:, :, 1] = 200
        arr[:, :, 2] = 200
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=150)

        flat_out = flat.reshape(h, stride, 4)
        assert np.all(flat_out[:w, :, 3] == 0)

    def test_threshold_boundary(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 200
        arr[:, :, 1] = 200
        arr[:, :, 2] = 200
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=201)

        flat_out = flat.reshape(h, stride, 4)
        brightness = (77 * 200 + 150 * 200 + 29 * 200) >> 8
        assert brightness == 200
        assert np.all(flat_out[:w, :, 3] == 255)

    def test_default_threshold(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 255
        arr[:, :, 1] = 255
        arr[:, :, 2] = 255
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride)

        flat_out = flat.reshape(h, stride, 4)
        assert np.all(flat_out[:w, :, 3] == 0)

    def test_preserves_color_channels(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 100
        arr[:, :, 1] = 150
        arr[:, :, 2] = 200
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_by_brightness(flat, w, h, stride, threshold=10)

        flat_out = flat.reshape(h, stride, 4)
        assert flat_out[0, 0, 0] == 100
        assert flat_out[0, 0, 1] == 150
        assert flat_out[0, 0, 2] == 200
        assert flat_out[0, 0, 3] == 0

    def test_weighted_brightness_not_average(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 1, 1)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((1, 1, 4))
        arr[0, 0, 0] = 255
        arr[0, 0, 1] = 0
        arr[0, 0, 2] = 0
        arr[0, 0, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        brightness = (77 * 0 + 150 * 0 + 29 * 255) >> 8
        assert brightness == 28

        make_transparent_by_brightness(flat, w, h, stride, threshold=29)
        flat_out = flat.reshape(h, stride, 4)
        assert flat_out[0, 0, 3] == 255

        make_transparent_by_brightness(flat, w, h, stride, threshold=28)
        flat_out = flat.reshape(h, stride, 4)
        assert flat_out[0, 0, 3] == 0


# ---------------------------------------------------------------------------
# make_transparent_except_color
# ---------------------------------------------------------------------------


class TestMakeTransparentExceptColor:
    def test_target_color_stays_opaque(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 100
        arr[:, :, 1] = 150
        arr[:, :, 2] = 200
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_except_color(
            flat, w, h, stride, target_r=200, target_g=150, target_b=100
        )

        flat_out = flat.reshape(h, stride, 4)
        assert np.all(flat_out[:w, :, 3] == 255)

    def test_non_matching_becomes_transparent(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 50
        arr[:, :, 1] = 100
        arr[:, :, 2] = 150
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_except_color(
            flat, w, h, stride, target_r=200, target_g=150, target_b=100
        )

        flat_out = flat.reshape(h, stride, 4)
        assert np.all(flat_out[:w, :, 3] == 0)

    def test_mixed_colors(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[0, 0, 0] = 0
        arr[0, 0, 1] = 255
        arr[0, 0, 2] = 0
        arr[0, 0, 3] = 255
        arr[0, 1, 0] = 255
        arr[0, 1, 1] = 0
        arr[0, 1, 2] = 0
        arr[0, 1, 3] = 255
        arr[1, 0, 0] = 0
        arr[1, 0, 1] = 255
        arr[1, 0, 2] = 0
        arr[1, 0, 3] = 255
        arr[1, 1, 0] = 0
        arr[1, 1, 1] = 0
        arr[1, 1, 2] = 255
        arr[1, 1, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_except_color(
            flat, w, h, stride, target_r=0, target_g=255, target_b=0
        )

        flat_out = flat.reshape(h, stride, 4)
        assert flat_out[0, 0, 3] == 255
        assert flat_out[0, 1, 3] == 0
        assert flat_out[1, 0, 3] == 255
        assert flat_out[1, 1, 3] == 0

    def test_preserves_color_channels(self):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 2, 2)
        data = surface.get_data()
        arr = np.frombuffer(data, dtype=np.uint8).reshape((2, 2, 4))
        arr[:, :, 0] = 50
        arr[:, :, 1] = 100
        arr[:, :, 2] = 200
        arr[:, :, 3] = 255
        surface.mark_dirty()

        flat, w, h, stride = _surface_to_flat(surface)
        make_transparent_except_color(
            flat, w, h, stride, target_r=255, target_g=0, target_b=0
        )

        flat_out = flat.reshape(h, stride, 4)
        assert flat_out[0, 0, 0] == 50
        assert flat_out[0, 0, 1] == 100
        assert flat_out[0, 0, 2] == 200
        assert flat_out[0, 0, 3] == 0

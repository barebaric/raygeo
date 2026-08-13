import numpy as np
import pytest

from raygeo.geo.algo.trace import find_external_contours


def _points_set(contour):
    return frozenset(tuple(p) for p in contour)


def test_single_pixel_component_dropped():
    # Contours with fewer than 3 points are dropped.
    image = np.zeros((5, 5), dtype=np.uint8)
    image[2, 2] = 1
    assert find_external_contours(image) == []


def test_two_components():
    image = np.zeros((10, 10), dtype=np.uint8)
    image[1:3, 1:3] = 1
    image[6:8, 6:8] = 1
    contours = find_external_contours(image)
    assert len(contours) == 2
    sets = {_points_set(c) for c in contours}
    assert sets == {
        frozenset(
            {(1.0, 1.0), (2.0, 1.0), (2.0, 2.0), (1.0, 2.0), (1.0, 1.0)}
        ),
        frozenset(
            {(6.0, 6.0), (7.0, 6.0), (7.0, 7.0), (6.0, 7.0), (6.0, 6.0)}
        ),
    }


def test_rect_component_returns_loop():
    image = np.zeros((8, 8), dtype=np.uint8)
    image[2:6, 3:7] = 1
    contours = find_external_contours(image)
    assert len(contours) == 1
    pts = contours[0]
    # Boundary of the 4x4 block: 12 distinct pixels plus the closing
    # point (first point repeated at the end).
    assert len(pts) == 13
    assert pts[0] == pts[-1]
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    assert min(xs) == 3 and max(xs) == 6
    assert min(ys) == 2 and max(ys) == 5


def test_all_background_returns_empty():
    image = np.zeros((5, 5), dtype=np.uint8)
    assert find_external_contours(image) == []


def test_all_foreground_returns_one_contour():
    image = np.ones((5, 5), dtype=np.uint8)
    contours = find_external_contours(image)
    assert len(contours) == 1
    assert len(contours[0]) == 17  # 16 boundary pixels + closing point


def test_diagonal_line():
    image = np.zeros((6, 6), dtype=np.uint8)
    for i in range(4):
        image[i, i] = 1
    contours = find_external_contours(image)
    assert len(contours) == 1
    assert len(contours[0]) >= 4


def test_contour_points_are_integer_pixel_coords():
    image = np.zeros((7, 7), dtype=np.uint8)
    image[1:6, 2:5] = 1
    contours = find_external_contours(image)
    for p in contours[0]:
        assert p[0] == int(p[0]) and p[1] == int(p[1])


def test_boolean_input_accepted():
    image = np.zeros((5, 5), dtype=bool)
    image[1:3, 1:3] = True
    contours = find_external_contours(image)
    assert len(contours) == 1


def test_ring_traces_outer_and_inner_boundaries():
    # A ring produces one contour for the outer boundary and one for
    # the hole boundary (the inner boundary pixels are foreground
    # pixels not visited by the outer trace).
    image = np.zeros((10, 10), dtype=np.uint8)
    image[1:9, 1:9] = 1
    image[3:7, 3:7] = 0
    contours = find_external_contours(image)
    assert len(contours) == 2
    outer = max(contours, key=len)
    xs = [p[0] for p in outer]
    ys = [p[1] for p in outer]
    assert min(xs) == 1 and max(xs) == 8
    assert min(ys) == 1 and max(ys) == 8


@pytest.mark.parametrize(
    "shape",
    [
        (1, 1),
        (10, 10),
        (100, 100),
    ],
)
def test_image_sizes(shape):
    image = np.zeros(shape, dtype=np.uint8)
    h, w = shape
    # A 2x2 block always yields a valid contour, except when the image
    # is too small to fit one (then the traced contour is dropped).
    image[h // 2 : h // 2 + 2, w // 2 : w // 2 + 2] = 1
    contours = find_external_contours(image)
    if h < 2 or w < 2:
        assert contours == []
    else:
        assert len(contours) == 1
        assert len(contours[0]) >= 3

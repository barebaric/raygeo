import numpy as np

from raygeo.geo import Geometry
from raygeo.geo.algo import hull


def test_get_enclosing_hull():
    boolean_image = np.full((100, 100), False, dtype=bool)
    boolean_image[10:30, 10:30] = True
    boolean_image[70:90, 70:90] = True

    geo = hull.get_enclosing_hull(boolean_image)

    assert geo is not None
    assert isinstance(geo, Geometry)
    assert len(geo) >= 5


def test_get_enclosing_hull_no_content():
    boolean_image = np.full((50, 50), False, dtype=bool)
    geo = hull.get_enclosing_hull(boolean_image)
    assert geo is None


def test_get_enclosing_hull_single_pixel():
    boolean_image = np.full((10, 10), False, dtype=bool)
    boolean_image[5, 5] = True
    geo = hull.get_enclosing_hull(boolean_image)
    assert geo is None


def test_get_enclosing_hull_single_component():
    boolean_image = np.full((50, 50), False, dtype=bool)
    boolean_image[10:40, 10:40] = True

    geo = hull.get_enclosing_hull(boolean_image)

    assert geo is not None
    assert isinstance(geo, Geometry)
    assert geo.area() > 0


def test_get_enclosing_hull_pixel_coords():
    boolean_image = np.zeros((100, 100), dtype=bool)
    boolean_image[20:80, 20:80] = True

    geo = hull.get_enclosing_hull(boolean_image)
    assert geo is not None

    data = geo.data
    for row in data:
        x, y = row.end[0], row.end[1]
        assert 20 <= x <= 79
        assert 20 <= y <= 79


def test_get_hulls_from_image_multiple_components():
    boolean_image = np.full((100, 100), False, dtype=bool)
    boolean_image[10:30, 10:30] = True
    boolean_image[70:90, 70:90] = True

    geometries = hull.get_hulls_from_image(boolean_image)

    assert isinstance(geometries, list)
    assert len(geometries) == 2
    assert all(isinstance(g, Geometry) for g in geometries)
    assert len(geometries[0]) >= 4
    assert len(geometries[1]) >= 4


def test_get_hulls_from_image_no_content():
    boolean_image = np.full((50, 50), False, dtype=bool)
    geometries = hull.get_hulls_from_image(boolean_image)
    assert isinstance(geometries, list)
    assert geometries == []


def test_get_hulls_from_image_single_component():
    boolean_image = np.full((50, 50), False, dtype=bool)
    boolean_image[10:40, 10:40] = True

    geometries = hull.get_hulls_from_image(boolean_image)

    assert len(geometries) == 1
    assert geometries[0].area() > 0


def test_concave_hull_creates_valid_indentation():
    height, width = 200, 200
    uint8_image = np.zeros((height, width), dtype=np.uint8)
    w_bottom, w_top, rect_h, gap, radius = 30, 150, 40, 20, 10
    top_y1 = (height - (2 * rect_h + gap)) // 2
    top_x1 = (width - w_top) // 2

    _fill_rounded_rect(
        uint8_image,
        (top_x1, top_y1),
        (top_x1 + w_top, top_y1 + rect_h),
        radius,
    )
    bottom_y1 = top_y1 + rect_h + gap
    bottom_x1 = (width - w_bottom) // 2
    _fill_rounded_rect(
        uint8_image,
        (bottom_x1, bottom_y1),
        (bottom_x1 + w_bottom, bottom_y1 + rect_h),
        radius,
    )

    boolean_image = uint8_image.astype(bool)
    convex_geo = hull.get_concave_hull(boolean_image, gravity=0.0)
    concave_geo = hull.get_concave_hull(boolean_image, gravity=0.9)

    assert convex_geo is not None
    assert concave_geo is not None
    assert len(concave_geo) > len(convex_geo)
    assert convex_geo.area() > 0
    assert concave_geo.area() < convex_geo.area()
    assert not concave_geo.has_self_intersections()

    # Shrinking the hull must not create intersections with the grown
    # convex hull.
    convex_geo.grow(1)
    assert not concave_geo.intersects_with(convex_geo)

    # The hull must enclose the original shape. The shape consists of
    # convex rounded rectangles, so each per-component hull describes
    # the component itself. Shrink each by a margin so that the hull
    # cannot merely touch them.
    shape_geometries = hull.get_hulls_from_image(boolean_image)
    assert len(shape_geometries) == 2
    for shape in shape_geometries:
        shape.grow(-2)
        assert concave_geo.encloses(shape)


def test_get_concave_hull_zero_gravity():
    boolean_image = np.zeros((100, 100), dtype=bool)
    boolean_image[20:80, 20:80] = True

    convex_geo = hull.get_enclosing_hull(boolean_image)
    concave_geo = hull.get_concave_hull(boolean_image, gravity=0.0)

    assert convex_geo is not None
    assert concave_geo is not None
    assert len(convex_geo) == len(concave_geo)
    for a, b in zip(convex_geo.data, concave_geo.data):
        assert type(a) is type(b)
        assert a.end == b.end


def test_get_concave_hull_no_content():
    boolean_image = np.full((50, 50), False, dtype=bool)
    geo = hull.get_concave_hull(boolean_image, gravity=0.5)
    assert geo is None


def test_enclosing_hull_y_not_flipped():
    boolean_image = np.zeros((100, 100), dtype=bool)
    boolean_image[10:20, 10:20] = True

    geo = hull.get_enclosing_hull(boolean_image)
    assert geo is not None

    data = geo.data
    for row in data:
        assert 10 <= row.end[1] <= 19


def _fill_rounded_rect(img, pt1, pt2, r):
    x1, y1 = pt1
    x2, y2 = pt2
    h, w = img.shape
    img[max(0, y1 + r) : min(h, y2 - r), max(0, x1) : min(w, x2)] = 255
    img[max(0, y1) : min(h, y2), max(0, x1 + r) : min(w, x2 - r)] = 255
    for cy, cx in [
        (y1 + r, x1 + r),
        (y1 + r, x2 - r),
        (y2 - r, x1 + r),
        (y2 - r, x2 - r),
    ]:
        yy, xx = np.ogrid[-r : r + 1, -r : r + 1]
        mask = xx**2 + yy**2 <= r**2
        img[cy - r : cy + r + 1, cx - r : cx + r + 1][mask] = 255

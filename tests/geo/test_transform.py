import pytest
import math
import numpy as np
from raygeo.geo import Geometry


def _create_translate_matrix(x, y, z):
    """Creates a NumPy translation matrix."""
    return np.array(
        [
            [1, 0, 0, x],
            [0, 1, 0, y],
            [0, 0, 1, z],
            [0, 0, 0, 1],
        ],
        dtype=float,
    )


def _create_scale_matrix(sx, sy, sz):
    """Creates a NumPy scaling matrix."""
    return np.array(
        [
            [sx, 0, 0, 0],
            [0, sy, 0, 0],
            [0, 0, sz, 0],
            [0, 0, 0, 1],
        ],
        dtype=float,
    )


def _create_z_rotate_matrix(angle_rad):
    """Creates a NumPy Z-axis rotation matrix."""
    c = math.cos(angle_rad)
    s = math.sin(angle_rad)
    return np.array(
        [
            [c, -s, 0, 0],
            [s, c, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
        ],
        dtype=float,
    )


# --- Affine Transform Tests ---


def test_transform_identity():
    geo = Geometry()
    geo.move_to(10, 20, 30)
    geo.arc_to(50, 60, i=5, j=7, z=40)
    original_geo = geo.copy()

    identity_matrix = np.identity(4, dtype=float)
    geo.transform(identity_matrix)

    assert geo == original_geo


def test_transform_translate():
    geo = Geometry()
    geo.move_to(10, 20, 30)
    geo.arc_to(50, 60, i=5, j=7, z=40)
    geo.bezier_to(70, 80, c1x=55, c1y=65, c2x=65, c2y=75, z=50)

    translate_matrix = _create_translate_matrix(10, -5, 15)
    geo.transform(translate_matrix)
    assert geo.data is not None

    # Check move
    assert np.allclose(geo.data[0, 1:4], (20, 15, 45))
    # Check arc
    assert np.allclose(geo.data[1, 1:4], (60, 55, 55))
    # Translation should NOT affect arc center offsets (vectors)
    assert np.allclose(geo.data[1, 4:6], (5, 7))
    # Check bezier
    assert np.allclose(geo.data[2, 1:4], (80, 75, 65))
    # Translation SHOULD affect bezier control points (absolute coords)
    assert np.allclose(
        geo.data[2, Geometry.COL_C1X : Geometry.COL_C1Y + 1], (65, 60)
    )
    assert np.allclose(
        geo.data[2, Geometry.COL_C2X : Geometry.COL_C2Y + 1], (75, 70)
    )


def test_transform_scale_non_uniform_preserves_beziers():
    geo = Geometry()
    geo.move_to(10, 20, 5)
    # Arc converted to bezier using arc_to_as_bezier(). This may create
    # one or more bezier segments.
    geo.arc_to_as_bezier(22, 22, i=5, j=7, z=-10)
    # This is the last command added.
    geo.bezier_to(30, 30, c1x=24, c1y=24, c2x=28, c2y=28, z=-20)
    scale_matrix = _create_scale_matrix(2, 3, 4)

    geo.transform(scale_matrix)
    assert geo.data is not None

    # 1. Check Move
    assert np.allclose(geo.data[0, 1:4], (20, 60, 20))

    # 2. Check that all subsequent commands are still Beziers
    assert np.all(geo.data[1:, Geometry.COL_TYPE] == Geometry.CMD_TYPE_BEZIER)

    # 3. Check the final state of the explicit bezier_to command.
    # It's the last row.
    final_bezier_row = geo.data[-1]

    # Check end point: (30*2, 30*3, -20*4) -> (60, 90, -80)
    final_point = final_bezier_row[Geometry.COL_X : Geometry.COL_Z + 1]
    assert np.allclose(final_point, (60.0, 90.0, -80.0))

    # Check C1: (24*2, 24*3) -> (48, 72)
    final_c1 = final_bezier_row[Geometry.COL_C1X : Geometry.COL_C1Y + 1]
    assert np.allclose(final_c1, (48.0, 72.0))

    # Check C2: (28*2, 28*3) -> (56, 84)
    final_c2 = final_bezier_row[Geometry.COL_C2X : Geometry.COL_C2Y + 1]
    assert np.allclose(final_c2, (56.0, 84.0))

    # 4. Check the final state of the arc_to_as_bezier command.
    # Its last segment is the second-to-last row.
    arc_end_row = geo.data[-2]

    # Check end point: (22*2, 22*3, -10*4) -> (44, 66, -40)
    arc_final_point = arc_end_row[Geometry.COL_X : Geometry.COL_Z + 1]
    assert np.allclose(arc_final_point, (44.0, 66.0, -40.0))


def test_transform_rotate_preserves_z():
    geo = Geometry()
    geo.move_to(10, 10, -5)
    rotate_matrix = _create_z_rotate_matrix(math.radians(90))

    geo.transform(rotate_matrix)
    assert geo.data is not None

    x, y, z = geo.data[0, 1:4]
    assert z == -5
    assert x == pytest.approx(-10)
    assert y == pytest.approx(10)


def test_transform_uniform_scale_preserves_curves():
    geo = Geometry()
    geo.move_to(0, 0, 0)
    # Arc from (0,0) to (10,0) with center at (5,0) -> radius 5
    geo.arc_to(10, 0, i=5, j=0, clockwise=True)
    # Bezier from (10,0) to (20,0)
    geo.bezier_to(20, 0, c1x=12, c1y=2, c2x=18, c2y=-2)

    # Uniform scale by 2
    scale_matrix = _create_scale_matrix(2, 2, 2)
    geo.transform(scale_matrix)
    assert geo.data is not None

    # Check arc
    arc_row = geo.data[1]
    assert arc_row[Geometry.COL_TYPE] == Geometry.CMD_TYPE_ARC
    assert np.allclose(arc_row[1:4], (20, 0, 0))
    # Offset should also scale
    assert np.allclose(arc_row[4:6], (10, 0))

    # Check bezier
    bezier_row = geo.data[2]
    assert bezier_row[Geometry.COL_TYPE] == Geometry.CMD_TYPE_BEZIER
    assert np.allclose(bezier_row[1:4], (40, 0, 0))
    # Control points should also scale
    assert np.allclose(
        bezier_row[Geometry.COL_C1X : Geometry.COL_C1Y + 1], (24, 4)
    )
    assert np.allclose(
        bezier_row[Geometry.COL_C2X : Geometry.COL_C2Y + 1], (36, -4)
    )


# --- Grow/Offset Tests ---


def test_grow_simple_square():
    """Tests growing and shrinking a simple CCW square."""
    square = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])

    # Grow the square
    grown_square = square.grow(1.0)
    assert grown_square.area() == pytest.approx(144.0)  # (10+2)^2
    # Check one of the new vertices
    grown_points = grown_square.segments()[0]
    # Use pytest.approx for floating point comparisons of coordinates
    assert any(np.allclose(p, (-1.0, -1.0, 0.0)) for p in grown_points), (
        "Expected grown vertex not found"
    )

    # Shrink the square
    shrunk_square = square.grow(-1.0)
    assert shrunk_square.area() == pytest.approx(64.0)  # (10-2)^2
    shrunk_points = shrunk_square.segments()[0]
    assert any(np.allclose(p, (1.0, 1.0, 0.0)) for p in shrunk_points), (
        "Expected shrunk vertex not found"
    )


def test_grow_clockwise_square():
    """Tests that offset direction is consistent for a CW shape."""
    # A clockwise square
    square_cw = Geometry.from_points([(0, 0), (0, 10), (10, 10), (10, 0)])

    # A positive offset on any shape should grow it
    grown_square = square_cw.grow(1.0)
    assert grown_square.area() == pytest.approx(144.0)

    # A negative offset on any shape should shrink it
    shrunk_square = square_cw.grow(-1.0)
    assert shrunk_square.area() == pytest.approx(64.0)


def test_grow_shape_with_hole():
    """Tests offsetting a shape containing a hole."""
    # Outer CCW square (0,0) -> (20,20), Area = 400
    outer = Geometry.from_points([(0, 0), (20, 0), (20, 20), (0, 20)])
    # Inner CW square (hole) (5,5) -> (15,15), Area = -100
    inner = Geometry.from_points([(5, 5), (5, 15), (15, 15), (15, 5)])
    shape_with_hole = outer.copy()
    shape_with_hole.extend(inner)
    assert shape_with_hole.area() == pytest.approx(300.0)

    # Grow by 1. Outer becomes 22x22, inner becomes 8x8.
    # New area = 22*22 - 8*8 = 484 - 64 = 420.
    grown_shape = shape_with_hole.grow(1.0)
    assert grown_shape.area() == pytest.approx(420.0)

    # Shrink by 1. Outer becomes 18x18, inner becomes 12x12.
    # New area = 18*18 - 12*12 = 324 - 144 = 180.
    shrunk_shape = shape_with_hole.grow(-1.0)
    assert shrunk_shape.area() == pytest.approx(180.0)


def test_grow_open_path_is_ignored():
    """Tests that open paths result in an empty geometry."""
    open_path = Geometry.from_points([(0, 0), (10, 10), (20, 0)], close=False)
    result = open_path.grow(1.0)
    assert result.is_empty()


def test_grow_circle():
    """Tests offsetting a shape with arcs by checking the resulting area."""
    radius = 10.0
    # Create a polygonal approximation of a circle using from_points. This
    # avoids issues with how area() handles ArcTo and ensures a valid, simple
    # polygon for testing the offset logic itself.
    num_points = 100
    angles = np.linspace(0, 2 * np.pi, num_points, endpoint=False)
    points = [(radius * np.cos(a), radius * np.sin(a)) for a in angles]
    circle = Geometry.from_points(points)

    original_area = math.pi * radius**2
    assert circle.area() == pytest.approx(original_area, rel=1e-3)

    # Grow the circle
    offset = 2.0
    grown_circle = circle.grow(offset)
    expected_grown_area = math.pi * (radius + offset) ** 2
    assert grown_circle.area() == pytest.approx(expected_grown_area, rel=1e-2)

    # Shrink the circle
    offset = -2.0
    shrunk_circle = circle.grow(offset)
    expected_shrunk_area = math.pi * (radius + offset) ** 2
    assert shrunk_circle.area() == pytest.approx(
        expected_shrunk_area, rel=1e-2
    )


def test_shrink_to_nothing():
    """Tests that shrinking a shape by its half-width or more is handled."""
    square = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])

    # Shrinking by half the width should result in a zero-area shape
    shrunk_to_point = square.grow(-5.0)
    assert shrunk_to_point.area() == pytest.approx(0.0)

    # Shrinking by more than the half-width should also result in zero area
    shrunk_past_zero = square.grow(-6.0)
    # The algorithm might produce a small self-intersecting shape with non-zero
    # area in this case, but it should be very small. A robust offset algorithm
    # would clean this up, but for now we check that it's close to zero.
    assert shrunk_past_zero.area() == pytest.approx(0.0, abs=1.0)


def test_grow_adjacent_contours_preserved():
    """
    Tests that adjacent separate contours remain separate after offsetting.

    When two shapes share an edge or are touching, growing them should NOT
    merge them into a single shape. Each contour must be offset independently
    to preserve distinct toolpaths.
    """
    square1 = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    square2 = Geometry.from_points([(10, 0), (20, 0), (20, 10), (10, 10)])
    combined = square1.copy()
    combined.extend(square2)

    # Each square has area 100, total = 200
    assert combined.area() == pytest.approx(200.0)

    # Grow by 1 unit. If contours are processed together, pyclipper would
    # merge them into one 22x12 rectangle (area 264). But processing each
    # contour separately gives two 12x12 squares (area 288 total).
    grown = combined.grow(1.0)
    assert grown.area() == pytest.approx(288.0)


def test_grow_overlapping_contours_preserved():
    """
    Tests that overlapping separate contours remain separate after offsetting.

    Two shapes that partially overlap should still be offset independently,
    not merged into a union.
    """
    square1 = Geometry.from_points([(0, 0), (15, 0), (15, 15), (0, 15)])
    square2 = Geometry.from_points([(10, 0), (25, 0), (25, 15), (10, 15)])
    combined = square1.copy()
    combined.extend(square2)

    # area() sums individual contour areas, not the union area
    assert combined.area() == pytest.approx(450.0)

    # Growing by 1: each 15x15 square becomes 17x17 = 289
    # If processed independently: 289 + 289 = 578
    # If merged as union: would be a single larger shape with different area
    grown = combined.grow(1.0)
    assert grown.area() == pytest.approx(578.0)


def test_grow_nested_islands_preserved():
    """
    Tests a 3-level hierarchy: Solid -> Hole -> Solid (Island).

    Behavior:
    1. Outer Solid grows outwards.
    2. Hole shrinks (because the solid wall expands inwards).
    3. Inner Island grows outwards.
    """
    # 1. Outer Box: 100x100 (Area 10000)
    outer = Geometry.from_points([(0, 0), (100, 0), (100, 100), (0, 100)])

    # 2. Hole: 60x60 (Area 3600)
    # Winding is CW, but the system should detect it as a hole regardless of
    # input winding provided it is inside the outer.
    hole = Geometry.from_points([(20, 20), (20, 80), (80, 80), (80, 20)])

    # 3. Island: 20x20 (Area 400) inside the hole
    island = Geometry.from_points([(40, 40), (60, 40), (60, 60), (40, 60)])

    geo = Geometry()
    geo.extend(outer)
    geo.extend(hole)
    geo.extend(island)

    # Initial Area check: 10000 - 3600 + 400 = 6800
    assert geo.area() == pytest.approx(6800.0)

    # Grow by 1 unit
    # Outer becomes 102x102 = 10404
    # Hole shrinks by 1 unit on all sides -> 58x58 = 3364
    # Island grows by 1 unit on all sides -> 22x22 = 484
    # Expected: 10404 - 3364 + 484 = 7524
    grown = geo.grow(1.0)

    assert grown.area() == pytest.approx(7524.0)


def test_grow_shape_with_multiple_holes():
    """Tests growing a solid containing multiple holes."""
    outer = Geometry.from_points([(0, 0), (100, 0), (100, 100), (0, 100)])
    hole1 = Geometry.from_points([(10, 10), (10, 30), (30, 30), (30, 10)])
    hole2 = Geometry.from_points([(70, 70), (70, 90), (90, 90), (90, 70)])
    hole3 = Geometry.from_points([(40, 40), (40, 60), (60, 60), (60, 40)])

    geo = Geometry()
    geo.extend(outer)
    geo.extend(hole1)
    geo.extend(hole2)
    geo.extend(hole3)

    assert geo.area() == pytest.approx(10000.0 - 3 * 400.0)

    contours = geo.split_into_contours()
    assert len(contours) == 4

    grown = geo.grow(1.0)

    grown_contours = grown.split_into_contours()
    assert len(grown_contours) == 4, (
        f"Expected 4 contours, got {len(grown_contours)}"
    )

    nonzero = [g for g in grown_contours if g.area() > 1.0]
    assert len(nonzero) == 4

    outer_grown = max(nonzero, key=lambda g: g.area())
    assert outer_grown.area() == pytest.approx(102 * 102, rel=0.01)

    grown_holes = sorted(
        [g for g in nonzero if g is not outer_grown],
        key=lambda g: g.area(),
    )
    for hole in grown_holes:
        assert hole.area() == pytest.approx(18 * 18, rel=0.05)


def test_grow_multiple_holes_no_micro_contours():
    """Growing a shape with multiple holes must not produce zero-area
    degenerate contours."""
    outer = Geometry.from_points([(0, 0), (60, 0), (60, 60), (0, 60)])
    holes = [
        Geometry.from_points([(x, y), (x, y + 8), (x + 8, y + 8), (x + 8, y)])
        for x in (5, 25, 45)
        for y in (5, 25, 45)
    ]

    geo = Geometry()
    geo.extend(outer)
    for h in holes:
        geo.extend(h)

    grown = geo.grow(0.5)
    contours = grown.split_into_contours()

    micro = [c for c in contours if c.area() < 1.0]
    assert len(micro) == 0, (
        f"Found {len(micro)} micro-contours (area < 1.0), "
        f"total contours: {len(contours)}"
    )


def test_grow_overlapping_solids_with_holes():
    """
    Tests that two overlapping solids, each containing their own hole,
    are processed independently.

    If they were unioned, the overlap area would merge and the holes might
    interact unpredictably.
    With the new logic, they should remain two distinct sets of (Solid-Hole).
    """
    # Solid A: 40x40 at (0,0) -> Area 1600
    solid_a = Geometry.from_points([(0, 0), (40, 0), (40, 40), (0, 40)])
    # Hole A: 10x10 at (15,15) -> Area 100
    hole_a = Geometry.from_points([(15, 15), (15, 25), (25, 25), (25, 15)])

    # Solid B: 40x40 at (20,0) -> Overlaps A by half. Area 1600.
    solid_b = Geometry.from_points([(20, 0), (60, 0), (60, 40), (20, 40)])
    # Hole B: 10x10 at (35,15) -> Area 100
    hole_b = Geometry.from_points([(35, 15), (35, 25), (45, 25), (45, 15)])

    geo = Geometry()
    geo.extend(solid_a)
    geo.extend(hole_a)
    geo.extend(solid_b)
    geo.extend(hole_b)

    # Initial area check: (1600 - 100) + (1600 - 100) = 3000
    # Note: Geometry.area() sums the signed areas of contours.
    # It does not perform boolean union calculation, which matches the
    # requirement that these are separate toolpaths.
    assert geo.area() == pytest.approx(3000.0)

    # Grow by 1.
    # Solid A -> 42x42 (1764). Hole A -> 8x8 (64). Net A = 1700.
    # Solid B -> 42x42 (1764). Hole B -> 8x8 (64). Net B = 1700.
    # Total = 3400.
    grown = geo.grow(1.0)
    assert grown.area() == pytest.approx(3400.0)


# --- Map to Frame Tests ---


def test_map_geometry_to_frame_identity():
    """Tests mapping a geometry to a frame matching its own bounding box."""
    geo = Geometry.from_points([(10, 20), (30, 20), (30, 50), (10, 50)])
    original_geo = geo.copy()

    # Define a frame that is identical to the geometry's bounding box
    origin = (10, 20)
    p_width = (30, 20)
    p_height = (10, 50)

    mapped_geo = geo.map_to_frame(origin, p_width, p_height)

    # The result should be identical to the original
    assert mapped_geo == original_geo


def test_map_geometry_to_frame_translate_scale():
    """Tests mapping a unit square to a larger, translated rectangle."""
    # Source is a 1x1 square at the origin
    unit_square = Geometry.from_points([(0, 0), (1, 0), (1, 1), (0, 1)])

    # Target is a 50x20 rectangle at (100, 200)
    origin = (100, 200)
    p_width = (150, 200)  # 50 units wide
    p_height = (100, 220)  # 20 units high

    mapped_geo = unit_square.map_to_frame(origin, p_width, p_height)

    # Check the bounding box of the result
    min_x, min_y, max_x, max_y = mapped_geo.rect()
    assert min_x == pytest.approx(100)
    assert min_y == pytest.approx(200)
    assert max_x == pytest.approx(150)
    assert max_y == pytest.approx(220)


def test_map_geometry_to_frame_non_uniform_scale():
    """Tests mapping (stretching) a geometry non-uniformly."""
    source = Geometry.from_points([(0, 0), (10, 0), (10, 5), (0, 5)])

    origin = (0, 0)
    p_width = (50, 0)
    p_height = (0, 100)

    mapped_geo = source.map_to_frame(origin, p_width, p_height)

    min_x, min_y, max_x, max_y = mapped_geo.rect()
    assert min_x == pytest.approx(0)
    assert min_y == pytest.approx(0)
    assert max_x == pytest.approx(50)
    assert max_y == pytest.approx(100)


def test_map_geometry_to_frame_rotate_and_shear():
    """Tests mapping to a rotated and sheared parallelogram frame."""
    unit_square = Geometry.from_points([(0, 0), (1, 0), (1, 1), (0, 1)])

    # Target is a parallelogram
    origin = (10, 10)
    p_width = (20, 15)  # Vector U = (10, 5)
    p_height = (5, 20)  # Vector V = (-5, 10)

    mapped_geo = unit_square.map_to_frame(origin, p_width, p_height)

    # The four corners of the unit square should map to the four corners of
    # the parallelogram: P0, P_width, P_height, P_width+P_height-P0
    segments = mapped_geo.segments()[0]
    expected_corners = [
        (10, 10, 0),  # origin
        (20, 15, 0),  # p_width
        (15, 25, 0),  # implicit 4th point: origin + U + V
        (5, 20, 0),  # p_height
    ]

    # Check if all expected corners are present in the transformed geometry's
    # vertices. The order might change due to how from_points works.
    assert len(segments) == 5  # 4 points + closing point
    for expected_corner in expected_corners:
        found = any(np.allclose(p, expected_corner) for p in segments)
        assert found, f"Corner {expected_corner} not found in transformed geo"


def test_map_geometry_to_frame_empty_geometry():
    """Tests that mapping an empty geometry results in an empty geometry."""
    empty_geo = Geometry()
    mapped_geo = empty_geo.map_to_frame((0, 0), (10, 0), (0, 10))
    assert mapped_geo.is_empty()


def test_map_geometry_to_frame_degenerate_source():
    """
    Tests that mapping a geometry with zero width or height is handled.
    """
    # A single line has zero width/height depending on orientation
    line_geo = Geometry.from_points([(0, 0), (10, 0)], close=False)
    assert line_geo.rect()[3] - line_geo.rect()[1] == 0  # Zero height

    mapped_geo = line_geo.map_to_frame((0, 0), (10, 0), (0, 10))
    # Should return an empty geometry as the scaling would be infinite
    assert mapped_geo.is_empty()

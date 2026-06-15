import math

import numpy as np
import pytest

from raygeo.geo import Arc, Bezier, Geometry, Line, Move
from raygeo.geo.shape.rect import get_combined_rect
from raygeo.svg import geometry_to_svg_path, parse_svg_path_data


@pytest.fixture
def empty_geometry():
    return Geometry()


@pytest.fixture
def sample_geometry():
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)
    geo.arc_to(20, 0, i=5, j=-10)
    return geo


def test_initialization(empty_geometry):
    assert len(empty_geometry) == 0
    assert empty_geometry.last_move_to == (0.0, 0.0, 0.0)
    assert empty_geometry.uniform_scalable is True


def test_add_commands(empty_geometry):
    empty_geometry.move_to(5, 5)
    assert len(empty_geometry) == 1
    empty_geometry.line_to(10, 10)
    assert len(empty_geometry) == 2
    assert isinstance(empty_geometry.data[0], Move)
    assert isinstance(empty_geometry.data[1], Line)


def test_simplify_wrapper(sample_geometry):
    """Tests that simplify returns a valid Geometry."""
    result = sample_geometry.simplify(tolerance=0.5)
    assert isinstance(result, Geometry)


def test_clear_commands(sample_geometry):
    sample_geometry.clear()
    assert len(sample_geometry) == 0
    assert sample_geometry.is_empty()


def test_move_to(sample_geometry):
    sample_geometry.move_to(15, 15)
    last_row = sample_geometry.data[-1]
    assert isinstance(last_row, Move)
    assert last_row.end == (15.0, 15.0, 0.0)


def test_line_to(sample_geometry):
    sample_geometry.line_to(20, 20)
    last_row = sample_geometry.data[-1]
    assert isinstance(last_row, Line)
    assert last_row.end == (20.0, 20.0, 0.0)


def test_close_path(sample_geometry):
    sample_geometry.move_to(5, 5, -1.0)
    sample_geometry.close_path()
    last_row = sample_geometry.data[-1]
    assert isinstance(last_row, Line)
    assert last_row.end == sample_geometry.last_move_to
    assert last_row.end == (5.0, 5.0, -1.0)


def test_arc_to():
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)
    geo.arc_to(20, 0, i=5, j=-10)
    geo.arc_to(5, 5, 2, 3, clockwise=False)
    last_row = geo.data[-1]
    assert isinstance(last_row, Arc)
    assert last_row.end == (5.0, 5.0, 0.0)
    assert last_row.clockwise is False


def test_bezier_to(empty_geometry):
    empty_geometry.bezier_to(10, 10, c1x=2, c1y=2, c2x=8, c2y=8)
    last_row = empty_geometry.data[-1]
    assert isinstance(last_row, Bezier)
    assert last_row.end == (10.0, 10.0, 0.0)
    assert last_row.control1 == (2.0, 2.0, 0.0)
    assert last_row.control2 == (8.0, 8.0, 0.0)


def test_serialization_deserialization(sample_geometry):
    geo_dict = sample_geometry.to_dict()
    new_geo = Geometry.from_dict(geo_dict)
    assert new_geo == sample_geometry


def test_copy_method(sample_geometry):
    """Tests the deep copy functionality of the Geometry class."""
    # Set a flag to ensure it's copied
    sample_geometry.uniform_scalable = False

    original_geo = sample_geometry
    copied_geo = original_geo.copy()

    # Check for initial equality and deep copy semantics
    assert copied_geo is not original_geo
    assert copied_geo == original_geo
    assert copied_geo.last_move_to == original_geo.last_move_to
    # Check that configuration flags are preserved
    assert copied_geo.uniform_scalable is False

    # Modify the original and check that the copy is unaffected
    original_geo.line_to(100, 100)
    assert copied_geo != original_geo
    assert len(copied_geo) == 3
    assert len(original_geo) == 4


def test_distance(sample_geometry):
    """
    Tests that the distance calculation correctly computes true arc length.
    """
    # The test must now expect the true distance, not an approximation.
    dist_line = math.hypot(10 - 0, 10 - 0)

    # Arc parameters for manual length calculation:
    # Center is (10,10) + (5,-10) = (15,0). Radius = dist from center to start.
    radius = math.hypot(10 - 15, 10 - 0)  # sqrt((-5)^2 + 10^2) = sqrt(125)
    start_angle = math.atan2(10 - 0, 10 - 15)  # atan2(10, -5)
    end_angle = math.atan2(0 - 0, 20 - 15)  # atan2(0, 5) -> 0

    # The default for arc_to is clockwise=True.
    # For a clockwise arc from a larger angle (start_angle in Q2) to a smaller
    # one (end_angle on the axis), the span is just the difference.
    angle_span = start_angle - end_angle
    dist_arc = radius * angle_span

    expected_dist = dist_line + dist_arc

    assert sample_geometry.distance() == pytest.approx(expected_dist)


def test_area():
    # Test case 1: Empty and open paths
    assert Geometry().area() == 0.0
    assert Geometry.from_points([(0, 0), (10, 10)], close=False).area() == 0.0

    # Test case 2: Simple 10x10 CCW square
    square = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    assert square.area() == pytest.approx(100.0)

    # Test case 3: Simple 10x10 CW square (should have same positive area)
    square_cw = Geometry.from_points([(0, 0), (0, 10), (10, 10), (10, 0)])
    assert square_cw.area() == pytest.approx(100.0)

    # Test case 4: Shape with a hole
    # Outer CCW square (0,0) -> (10,10)
    geo_with_hole = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    # Inner CW square (hole) (2,2) -> (8,8)
    hole = Geometry.from_points([(2, 2), (2, 8), (8, 8), (8, 2)])
    geo_with_hole.extend(hole)  # Use extend to merge numpy data
    # Expected area = 100 - (6*6) = 64
    assert geo_with_hole.area() == pytest.approx(64.0)

    # Test case 5: Two separate shapes
    geo_two_shapes = Geometry.from_points([(0, 0), (5, 0), (5, 5), (0, 5)])
    second_shape = Geometry.from_points(
        [(10, 10), (15, 10), (15, 15), (10, 15)]
    )
    geo_two_shapes.extend(second_shape)  # Use extend to merge numpy data
    # Expected area = 25 + 25 = 50
    assert geo_two_shapes.area() == pytest.approx(50.0)


def test_segments():
    """Tests the segments() method for extracting point lists."""
    # Test case 1: Empty geometry
    geo_empty = Geometry()
    assert geo_empty.segments() == []

    # Test case 2: Single open path
    geo_open = Geometry()
    geo_open.move_to(0, 0, 1)
    geo_open.line_to(10, 0, 2)
    geo_open.arc_to(10, 10, i=0, j=5, z=3)
    expected_open = [[(0, 0, 1), (10, 0, 2), (10, 10, 3)]]
    assert geo_open.segments() == expected_open

    # Test case 3: Single closed path
    geo_closed = Geometry.from_points([(0, 0), (10, 0), (0, 10)])
    expected_closed = [[(0, 0, 0), (10, 0, 0), (0, 10, 0), (0, 0, 0)]]
    assert geo_closed.segments() == expected_closed

    # Test case 4: Multiple disjoint segments
    geo_multi = Geometry()
    # Segment 1
    geo_multi.move_to(0, 0)
    geo_multi.line_to(1, 1)
    # Segment 2
    geo_multi.move_to(10, 10)
    geo_multi.line_to(11, 11)
    geo_multi.line_to(12, 12)
    expected_multi = [
        [(0, 0, 0), (1, 1, 0)],
        [(10, 10, 0), (11, 11, 0), (12, 12, 0)],
    ]
    assert geo_multi.segments() == expected_multi

    # Test case 5: Path starting with a LineTo (implicit start at 0,0,0)
    geo_implicit_start = Geometry()
    geo_implicit_start.line_to(5, 5)
    geo_implicit_start.line_to(10, 0)
    expected_implicit = [[(0, 0, 0), (5, 5, 0), (10, 0, 0)]]
    assert geo_implicit_start.segments() == expected_implicit


def test_from_points():
    """Tests the Geometry.from_points classmethod."""
    # Test case 1: Empty list
    geo_empty = Geometry.from_points([])
    assert geo_empty.is_empty()

    # Test case 2: Single point
    geo_single = Geometry.from_points([(10, 20)])
    assert len(geo_single) == 1
    assert isinstance(geo_single.data[0], Move)
    assert geo_single.data[0].end == (10, 20, 0)
    assert geo_single.last_move_to == (10, 20, 0)

    # Test case 3: Triangle (closed by default)
    points = [(0, 0), (10, 0), (5, 10)]
    geo_triangle = Geometry.from_points(points)
    assert len(geo_triangle) == 4
    assert isinstance(geo_triangle.data[0], Move)
    assert geo_triangle.data[0].end == (0, 0, 0)
    assert isinstance(geo_triangle.data[3], Line)
    assert geo_triangle.data[3].end == (0, 0, 0)

    # Test case 4: Triangle (open)
    geo_triangle_open = Geometry.from_points(points, close=False)
    assert len(geo_triangle_open) == 3
    assert geo_triangle_open.data[-1].end != geo_triangle_open.data[0].end

    # Test case 5: Points with Z coordinates (closed)
    points_3d = [(0, 0, 1), (10, 0, 2), (5, 10, 3)]
    geo_3d = Geometry.from_points(points_3d)
    assert len(geo_3d) == 4
    assert geo_3d.data[0].end == (0, 0, 1)
    assert geo_3d.data[3].end == (0, 0, 1)
    assert geo_3d.last_move_to == (0, 0, 1)


def test_to_dict_and_from_dict(sample_geometry):
    """
    Tests the to_dict() and from_dict() methods for serialization.
    """
    # Test with a non-empty geometry
    geo_with_bezier = Geometry()
    geo_with_bezier.move_to(0, 0)
    geo_with_bezier.line_to(10, 10)
    geo_with_bezier.arc_to(20, 0, i=5, j=-10)
    geo_with_bezier.bezier_to(30, 10, c1x=22, c1y=2, c2x=28, c2y=8)
    dict_data = geo_with_bezier.to_dict()
    loaded_geo = Geometry.from_dict(dict_data)

    assert dict_data["last_move_to"] == list(geo_with_bezier.last_move_to)
    assert len(dict_data["commands"]) == 4
    assert dict_data["commands"][0] == ["M", 0.0, 0.0, 0.0]
    assert dict_data["commands"][1] == ["L", 10.0, 10.0, 0.0]
    assert dict_data["commands"][2] == ["A", 20.0, 0.0, 0.0, 5.0, -10.0, 1]
    assert dict_data["commands"][3] == [
        "B",
        30.0,
        10.0,
        0.0,
        22,
        2,
        0.0,
        28,
        8,
        0.0,
    ]

    assert loaded_geo == geo_with_bezier

    # Test with an empty geometry
    empty_geo = Geometry()
    dict_empty = empty_geo.to_dict()
    loaded_empty = Geometry.from_dict(dict_empty)

    assert dict_empty["last_move_to"] == [0.0, 0.0, 0.0]
    assert dict_empty["commands"] == []
    assert loaded_empty.is_empty()
    assert loaded_empty.last_move_to == (0.0, 0.0, 0.0)


def test_to_dict_arc_roundtrip_preserves_geometry():
    """Arc roundtrip through dict preserves geometry."""
    for d in [
        "M 0.25 0.5 A 0.25 0.25 0 1 1 0.75 0.5",
        "M 0.25 0.5 A 0.25 0.25 0 0 1 0.75 0.5",
    ]:
        orig = parse_svg_path_data(d)[0]
        dict_data = orig.to_dict()
        restored = Geometry.from_dict(dict_data)
        assert restored == orig


def test_arc_cw_export_string_match():
    """Constructed CW arc exported string is predictable."""
    geo = Geometry()
    geo.move_to(0.5, 0.5, 0.0)
    geo.arc_to(1.0, 1.0, i=0.5, j=0.0, clockwise=True)
    path = geometry_to_svg_path(geo, 100, 100)
    assert path == "M 50.000 50.000 A 50.000 50.000 0 0 1 100.000 0.000"


def test_arc_ccw_export_string_match():
    """Constructed CCW arc exported string is predictable (large-arc=1)."""
    geo = Geometry()
    geo.move_to(0.5, 0.5, 0.0)
    geo.arc_to(1.0, 1.0, i=0.5, j=0.0, clockwise=False)
    path = geometry_to_svg_path(geo, 100, 100)
    assert path == "M 50.000 50.000 A 50.000 50.000 0 1 0 100.000 0.000"


def test_map_to_frame_wrapper():
    """Tests that map_to_frame() returns a Geometry."""
    geo = Geometry.from_points([(0, 0), (1, 1)])
    origin = (10, 10)
    p_width = (20, 10)
    p_height = (10, 30)
    result = geo.map_to_frame(origin, p_width, p_height)
    assert isinstance(result, Geometry)


# --- Force Bezier Conversion Tests ---


def test_force_beziers_init():
    """Test that the configuration flag is stored correctly."""
    geo = Geometry()
    assert geo.uniform_scalable is True


def test_arc_to_as_bezier():
    """Test that arc_to_as_bezier creates Bezier commands."""
    geo = Geometry()
    geo.move_to(0, 0)

    # Create a simple 90 degree arc
    # Start (0,0), Center (10,0), End (10,10)
    # i=10, j=0. Clockwise=False (CCW)
    geo.arc_to_as_bezier(10, 10, i=10, j=0, clockwise=False)

    data = geo.data

    assert len(data) > 1
    assert isinstance(data[0], Move)

    # Check that subsequent commands are BEZIER, not ARC
    for i in range(1, len(data)):
        assert isinstance(data[i], Bezier)

    # Check endpoints of the last segment match the requested arc end
    last_row = data[-1]
    assert math.isclose(last_row.end[0], 10.0)
    assert math.isclose(last_row.end[1], 10.0)


def test_extend_preserves_uniform_scalable():
    """
    Test that extending a geometry preserves the uniform_scalable flag.
    """
    # Source geometry with an arc
    source = Geometry()
    source.move_to(0, 0)
    source.arc_to(10, 0, i=5, j=0, clockwise=True)  # Semi-circle

    # Destination geometry
    dest = Geometry()
    dest.extend(source)

    data = dest.data

    # Should contain Moves and Arcs
    assert any(isinstance(c, Move) for c in data)
    assert any(isinstance(c, Arc) for c in data)
    assert not dest.uniform_scalable


def test_from_dict_preserves_uniform_scalable(sample_geometry):
    """
    Test that from_dict preserves the uniform_scalable flag.
    """
    # sample_geometry contains an arc
    dict_data = sample_geometry.to_dict()

    # Load
    loaded = Geometry.from_dict(dict_data)

    data = loaded.data

    assert any(isinstance(c, Arc) for c in data)
    assert not loaded.uniform_scalable

    # Check that endpoint is preserved
    last_row = data[-1]
    # sample_geometry ends at (20, 0)
    assert math.isclose(last_row.end[0], 20.0)
    assert math.isclose(last_row.end[1], 0.0)


# --- Wrapper Method Tests ---
# These tests verify that the Geometry methods correctly wrap and call the
# underlying stateless functions from other modules.


def test_close_gaps_wrapper(sample_geometry):
    """Tests the Geometry.close_gaps() wrapper method."""
    result = sample_geometry.close_gaps(tolerance=1e-5)
    assert result is sample_geometry


def test_cleanup_wrapper(sample_geometry):
    """Tests the Geometry.cleanup() wrapper method."""
    result = sample_geometry.cleanup(tolerance=1e-5)
    assert result is sample_geometry


def test_split_inner_and_outer_contours_wrapper(sample_geometry):
    """Tests the Geometry.split_inner_and_outer_contours() wrapper method."""
    result = sample_geometry.split_inner_and_outer_contours()
    assert isinstance(result, tuple)
    assert len(result) == 2


def test_is_closed_wrapper(sample_geometry):
    """Tests the Geometry.is_closed() wrapper method."""
    result = sample_geometry.is_closed(tolerance=1e-5)
    assert isinstance(result, bool)


def test_remove_inner_edges_wrapper(sample_geometry):
    """Tests the Geometry.remove_inner_edges() wrapper method."""
    result = sample_geometry.remove_inner_edges()
    assert isinstance(result, Geometry)


def test_split_into_contours_wrapper(sample_geometry):
    """Tests the Geometry.split_into_contours() wrapper method."""
    result = sample_geometry.split_into_contours()
    assert isinstance(result, list)


def test_split_into_components_wrapper(sample_geometry):
    """Tests the Geometry.split_into_components() wrapper method."""
    result = sample_geometry.split_into_components()
    assert isinstance(result, list)


def test_encloses_wrapper(sample_geometry):
    """Tests the Geometry.encloses() wrapper method."""
    other_geo = Geometry()
    result = sample_geometry.encloses(other_geo)
    assert isinstance(result, bool)


def test_has_self_intersections_wrapper(sample_geometry):
    """Tests the Geometry.has_self_intersections() wrapper method."""
    result = sample_geometry.has_self_intersections(fail_on_t_junction=True)
    assert isinstance(result, bool)


def test_intersects_with_wrapper(sample_geometry):
    """Tests the Geometry.intersects_with() wrapper method."""
    other_geo = Geometry()
    other_geo.line_to(1, 1)
    result = sample_geometry.intersects_with(other_geo)
    assert isinstance(result, bool)


def test_grow_wrapper(sample_geometry):
    """Tests the Geometry.grow() wrapper method."""
    result = sample_geometry.grow(amount=5.0)
    assert isinstance(result, Geometry)


def test_transform_wrapper(sample_geometry):
    """
    Tests that Geometry.transform() correctly transforms geometry data.
    """
    matrix = np.identity(4)
    sample_geometry.transform(matrix)
    # Identity matrix should leave geometry unchanged
    assert len(sample_geometry.data) == 3


def test_flip_x():
    """Tests that flip_x() correctly inverts X coordinates."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)
    geo.arc_to(20, 5, i=5, j=-5, clockwise=False)
    geo.bezier_to(30, 10, c1x=22, c1y=2, c2x=28, c2y=8)

    # Save original data
    original_data = geo.data[:]

    # Apply flip_x
    result = geo.flip_x()

    assert result is geo

    # Check that X coordinates are inverted
    # MoveTo: (0, 0) -> (0, 0)
    assert math.isclose(geo.data[0].end[0], 0.0)
    assert math.isclose(geo.data[0].end[1], 0.0)

    # LineTo: (10, 10) -> (-10, 10)
    assert math.isclose(geo.data[1].end[0], -10.0)
    assert math.isclose(geo.data[1].end[1], 10.0)

    # ArcTo: (20, 5) -> (-20, 5), I=5 -> -5, CW toggled
    assert isinstance(geo.data[2], Arc)
    assert math.isclose(geo.data[2].end[0], -20.0)
    assert math.isclose(geo.data[2].end[1], 5.0)
    assert math.isclose(geo.data[2].center_offset[0], -5.0)
    assert geo.data[2].clockwise is True

    # BezierTo: (30, 10) -> (-30, 10), C1X=22 -> -22, C2X=28 -> -28
    assert isinstance(geo.data[3], Bezier)
    assert math.isclose(geo.data[3].end[0], -30.0)
    assert math.isclose(geo.data[3].end[1], 10.0)
    assert math.isclose(geo.data[3].control1[0], -22.0)
    assert math.isclose(geo.data[3].control2[0], -28.0)

    # Flipping twice should return to original
    geo.flip_x()
    for cmd, orig in zip(geo.data, original_data):
        assert type(cmd) is type(orig)
        np.testing.assert_allclose(cmd.end, orig.end, atol=1e-9)


def test_flip_y():
    """Tests that flip_y() correctly inverts Y coordinates."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)
    geo.arc_to(20, 5, i=5, j=-5, clockwise=False)
    geo.bezier_to(30, 10, c1x=22, c1y=2, c2x=28, c2y=8)

    # Save original data
    original_data = geo.data[:]

    # Apply flip_y
    result = geo.flip_y()

    assert result is geo

    # Check that Y coordinates are inverted
    # MoveTo: (0, 0) -> (0, 0)
    assert math.isclose(geo.data[0].end[0], 0.0)
    assert math.isclose(geo.data[0].end[1], 0.0)

    # LineTo: (10, 10) -> (10, -10)
    assert math.isclose(geo.data[1].end[0], 10.0)
    assert math.isclose(geo.data[1].end[1], -10.0)

    # ArcTo: (20, 5) -> (20, -5), J=-5 -> 5, CW toggled
    assert isinstance(geo.data[2], Arc)
    assert math.isclose(geo.data[2].end[0], 20.0)
    assert math.isclose(geo.data[2].end[1], -5.0)
    assert math.isclose(geo.data[2].center_offset[1], 5.0)
    assert geo.data[2].clockwise is True

    # BezierTo: (30, 10) -> (30, -10), C1Y=2 -> -2, C2Y=8 -> -8
    assert isinstance(geo.data[3], Bezier)
    assert math.isclose(geo.data[3].end[0], 30.0)
    assert math.isclose(geo.data[3].end[1], -10.0)
    assert math.isclose(geo.data[3].control1[1], -2.0)
    assert math.isclose(geo.data[3].control2[1], -8.0)

    # Flipping twice should return to original
    geo.flip_y()
    for cmd, orig in zip(geo.data, original_data):
        assert type(cmd) is type(orig)
        np.testing.assert_allclose(cmd.end, orig.end, atol=1e-9)


def test_get_command_at_valid_index():
    """Tests get_command_at() with valid indices."""
    geo = Geometry()
    geo.move_to(0, 0, 1)
    geo.line_to(10, 10, 2)
    geo.arc_to(20, 0, i=5, j=-10, clockwise=False, z=3)
    geo.bezier_to(30, 10, c1x=22, c1y=2, c2x=28, c2y=8, z=4)

    cmd0 = geo.get_command_at(0)
    assert isinstance(cmd0, Move)
    assert cmd0.end == (0.0, 0.0, 1.0)

    cmd1 = geo.get_command_at(1)
    assert isinstance(cmd1, Line)
    assert cmd1.end == (10.0, 10.0, 2.0)

    cmd2 = geo.get_command_at(2)
    assert isinstance(cmd2, Arc)
    assert cmd2.end == (20.0, 0.0, 3.0)
    assert cmd2.center_offset == (5.0, -10.0)

    cmd3 = geo.get_command_at(3)
    assert isinstance(cmd3, Bezier)
    assert cmd3.end == (30.0, 10.0, 4.0)
    assert cmd3.control1 == (22.0, 2.0, 0.0)
    assert cmd3.control2 == (28.0, 8.0, 0.0)


def test_get_command_at_negative_index():
    """Tests get_command_at() with negative index."""
    geo = Geometry()
    geo.move_to(0, 0)
    assert geo.get_command_at(-1) is None


def test_get_command_at_out_of_bounds():
    """Tests get_command_at() with index out of bounds."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)
    assert geo.get_command_at(2) is None
    assert geo.get_command_at(100) is None


def test_get_command_at_empty_geometry():
    """Tests get_command_at() on empty geometry."""
    geo = Geometry()
    assert geo.get_command_at(0) is None


def test_iter_commands():
    """Tests iter_commands() yields all commands correctly."""
    geo = Geometry()
    geo.move_to(0, 0, 1)
    geo.line_to(10, 10, 2)
    geo.arc_to(20, 0, i=5, j=-10, clockwise=False, z=3)
    geo.bezier_to(30, 10, c1x=22, c1y=2, c2x=28, c2y=8, z=4)

    commands = list(geo.iter_commands())

    assert len(commands) == 4
    assert isinstance(commands[0], Move)
    assert commands[0].end == (0.0, 0.0, 1.0)
    assert isinstance(commands[1], Line)
    assert commands[1].end == (10.0, 10.0, 2.0)
    assert isinstance(commands[2], Arc)
    assert commands[2].end == (20.0, 0.0, 3.0)
    assert commands[2].center_offset == (5.0, -10.0)
    assert isinstance(commands[3], Bezier)
    assert commands[3].end == (30.0, 10.0, 4.0)
    assert commands[3].control1 == (22.0, 2.0, 0.0)
    assert commands[3].control2 == (28.0, 8.0, 0.0)


def test_iter_commands_empty_geometry():
    """Tests iter_commands() on empty geometry."""
    geo = Geometry()
    commands = list(geo.iter_commands())
    assert commands == []


def test_iter_commands_clockwise_arc():
    """Tests iter_commands() with clockwise arc."""
    geo = Geometry()
    geo.move_to(10, 10)
    geo.arc_to(15, 10, i=0, j=-5, clockwise=True)

    commands = list(geo.iter_commands())

    assert len(commands) == 2
    assert isinstance(commands[0], Move)
    assert commands[0].end == (10.0, 10.0, 0.0)
    assert isinstance(commands[1], Arc)
    assert commands[1].end == (15.0, 10.0, 0.0)
    assert commands[1].center_offset == (0.0, -5.0)
    assert commands[1].clockwise is True


def test_upgrade_to_scalable_on_scalable_geo():
    """
    Tests that upgrade_to_scalable does nothing on an already scalable
    geometry.
    """
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)
    geo.bezier_to(20, 0, c1x=12, c1y=12, c2x=18, c2y=2)
    original_data = geo.data[:]

    assert geo.uniform_scalable is True
    result = geo.upgrade_to_scalable()

    assert result is geo
    assert geo.uniform_scalable is True
    for cmd, orig in zip(geo.data, original_data):
        assert type(cmd) is type(orig)
        assert cmd.end == orig.end


def test_upgrade_to_scalable_on_empty_geo():
    """Tests that upgrade_to_scalable handles empty geometry gracefully."""
    geo = Geometry()
    assert geo.is_empty()
    assert geo.uniform_scalable is True

    result = geo.upgrade_to_scalable()

    assert result is geo
    assert geo.is_empty()
    assert geo.uniform_scalable is True


def test_upgrade_to_scalable_converts_arcs():
    """Tests the core functionality of converting arcs to beziers."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)  # Preserved command
    geo.arc_to(20, 0, i=5, j=-10, clockwise=True)  # Command to be converted

    # Initial state check
    assert geo.uniform_scalable is False

    # Perform the upgrade
    result = geo.upgrade_to_scalable()
    assert result is geo

    # Final state check
    assert geo.uniform_scalable is True

    # Check command types
    assert not any(isinstance(c, Arc) for c in geo.data)
    assert any(isinstance(c, Bezier) for c in geo.data)
    assert any(isinstance(c, Line) for c in geo.data)
    assert any(isinstance(c, Move) for c in geo.data)

    # Check path integrity
    # The last bezier should end where the arc ended.
    np.testing.assert_allclose(geo.data[-1].end, [20.0, 0.0, 0.0], atol=1e-9)


def test_upgrade_to_scalable_multiple_arcs():
    """Tests upgrading a geometry with multiple arcs and segments."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.arc_to(10, 10, i=10, j=0, clockwise=False)  # 90 deg CCW arc
    geo.line_to(20, 10)
    geo.arc_to(30, 0, i=0, j=-10, clockwise=True)  # 90 deg CW arc
    final_end_point = (30.0, 0.0, 0.0)

    assert geo.uniform_scalable is False

    geo.upgrade_to_scalable()

    assert geo.uniform_scalable is True
    assert not any(isinstance(c, Arc) for c in geo.data)

    # Verify that the final endpoint of the entire path is preserved
    np.testing.assert_allclose(geo.data[-1].end, final_end_point, atol=1e-9)


def test_upgrade_to_scalable_is_idempotent():
    """
    Tests that calling upgrade_to_scalable multiple times has no extra effect.
    """
    geo = Geometry()
    geo.move_to(0, 0)
    geo.arc_to(10, 10, i=10, j=0, clockwise=False)

    # First call
    geo.upgrade_to_scalable()
    data_after_first_call = geo.data[:]
    assert geo.uniform_scalable is True
    assert not any(isinstance(c, Arc) for c in data_after_first_call)

    # Second call
    geo.upgrade_to_scalable()
    data_after_second_call = geo.data

    # The data should be identical
    for cmd, orig in zip(data_after_second_call, data_after_first_call):
        assert type(cmd) is type(orig)
        assert cmd.end == orig.end
        if isinstance(cmd, Bezier):
            assert cmd.control1 == orig.control1
            assert cmd.control2 == orig.control2


def test_linearize_approximation():
    """
    Test that linearize() converts curves to lines within tolerance.
    """
    geo = Geometry()
    geo.move_to(0, 0)
    # 90 degree arc of radius 10
    geo.arc_to(10, 10, i=10, j=0, clockwise=False)

    # Use a coarse tolerance to see visible simplification
    geo.linearize(tolerance=0.1)

    # Should contain no Arcs
    assert not any(isinstance(c, Arc) for c in geo.data)
    # Should contain Lines
    assert any(isinstance(c, Line) for c in geo.data)

    # The end point should still be (10, 10)
    np.testing.assert_allclose(geo.data[-1].end, (10.0, 10.0, 0.0), atol=1e-6)


def test_fit_arcs_approximation():
    """
    Test that fit_arcs() reconstructs arcs from dense points.
    """
    # Create a dense polyline that approximates a circle
    points = []
    radius = 10.0
    center = (0.0, 0.0)
    for i in range(101):
        angle = (i / 100.0) * (np.pi / 2)  # 0 to 90 degrees
        x = center[0] + radius * math.cos(angle)
        y = center[1] + radius * math.sin(angle)
        points.append((x, y, 0.0))

    geo = Geometry.from_points(points, close=False)

    # Before fitting, it should be all Lines
    assert all(isinstance(c, Line) for c in geo.data[1:])

    # Fit arcs with a reasonable tolerance
    geo.fit_arcs(tolerance=0.1)

    # After fitting, it should contain Arcs
    assert any(isinstance(c, Arc) for c in geo.data)

    # Should have significantly fewer commands than 100 lines
    assert len(geo.data) < 10


def test_fit_arcs_mixed_geometry():
    """
    Test fitting on geometry that has both straight lines and curves.
    """
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)  # Straight line
    geo.arc_to(20, 10, i=0, j=10, clockwise=False)  # 90 deg arc
    original_data = geo.data[:]

    # Fitting arcs should process both the line and the arc segments.
    # Note: Because the fitting process is not perfectly lossless (it relies on
    # heuristics and tolerances), splitting may occur or very minor floating
    # point differences may be introduced. We check for semantic equivalence
    # rather than strict binary equality.
    geo.fit_arcs(tolerance=0.1)

    # 1. Verify the endpoints are preserved
    # The last point should still be (20, 10, 0)
    np.testing.assert_allclose(
        geo.data[-1].end, original_data[-1].end, atol=1e-6
    )

    # 2. Verify we still have Arcs and Lines
    assert any(isinstance(c, Arc) for c in geo.data)
    assert any(isinstance(c, Line) for c in geo.data)

    # 3. Verify the number of commands didn't explode (it shouldn't degrade
    # to polylines). Original was 3 commands (Move, Line, Arc).
    # Result should be close to that (e.g., <= 5 if the arc got split).
    assert len(geo.data) <= 5


def test_fit_curves_preserves_beziers():
    geo = Geometry()
    geo.move_to(0, 0)
    geo.bezier_to(10, 10, c1x=2, c1y=5, c2x=8, c2y=5)
    geo.line_to(15, 10)

    geo.fit_curves(tolerance=0.1, beziers=True, arcs=True)

    assert any(isinstance(c, Bezier) for c in geo.data)


def test_fit_curves_delegates_to_fit_arcs():
    geo = Geometry()
    geo.move_to(0, 0)
    for i in range(100):
        angle = 2 * math.pi * i / 100
        geo.line_to(10 * math.cos(angle), 10 * math.sin(angle))

    geo.fit_arcs(tolerance=0.1)

    assert any(isinstance(c, Arc) for c in geo.data)


class TestToPolygons:
    def test_empty_geometry(self):
        geo = Geometry()
        polygons = geo.to_polygons()
        assert polygons == []

    def test_single_triangle(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(5, 10)
        geo.close_path()

        polygons = geo.to_polygons()
        assert len(polygons) == 1
        assert len(polygons[0]) >= 3

    def test_single_square(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)
        geo.line_to(0, 10)
        geo.close_path()

        polygons = geo.to_polygons()
        assert len(polygons) == 1
        assert len(polygons[0]) >= 3

    def test_multiple_segments(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)
        geo.line_to(0, 10)
        geo.close_path()
        geo.move_to(20, 0)
        geo.line_to(30, 0)
        geo.line_to(30, 10)
        geo.line_to(20, 10)
        geo.close_path()

        polygons = geo.to_polygons()
        assert len(polygons) == 2

    def test_with_arc(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.arc_to(10, 10, 0, 5, clockwise=False)
        geo.line_to(0, 10)
        geo.close_path()

        polygons = geo.to_polygons(tolerance=0.5)
        assert len(polygons) == 1
        assert len(polygons[0]) >= 3

    def test_open_path(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)

        polygons = geo.to_polygons()
        assert len(polygons) == 1

    def test_tolerance_affects_cleaning(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)
        geo.line_to(0, 10)
        geo.close_path()

        polygons_low_tol = geo.to_polygons(tolerance=0.01)
        polygons_high_tol = geo.to_polygons(tolerance=1.0)

        assert len(polygons_low_tol) == 1
        assert len(polygons_high_tol) == 1

    def test_nested_geometry(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(20, 0)
        geo.line_to(20, 20)
        geo.line_to(0, 20)
        geo.close_path()
        geo.move_to(5, 5)
        geo.line_to(15, 5)
        geo.line_to(15, 15)
        geo.line_to(5, 15)
        geo.close_path()

        polygons = geo.to_polygons()
        assert len(polygons) == 2


class TestFilter:
    def test_filter_keep_all(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.arc_to(10, 10, i=5, j=0)
        geo.line_to(0, 10)

        filtered = geo.filter({0, 1, 2, 3})
        assert len(filtered) == 4
        assert isinstance(filtered.data[0], Move)
        assert isinstance(filtered.data[1], Line)
        assert isinstance(filtered.data[2], Arc)
        assert isinstance(filtered.data[3], Line)

    def test_filter_keep_subset(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.arc_to(10, 10, i=5, j=0)
        geo.line_to(0, 10)

        filtered = geo.filter({0, 2})
        assert len(filtered) == 2
        assert isinstance(filtered.data[0], Move)
        assert isinstance(filtered.data[1], Arc)
        assert filtered.data[1].end[0] == pytest.approx(10.0)
        assert filtered.data[1].end[1] == pytest.approx(10.0)
        assert filtered.data[1].center_offset[0] == pytest.approx(5.0)

    def test_filter_empty_indices(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)

        filtered = geo.filter(set())
        assert len(filtered) == 0

    def test_filter_preserves_original(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)

        filtered = geo.filter({0})
        assert len(geo) == 3
        assert len(filtered) == 1
        assert isinstance(filtered.data[0], Move)


class TestSegmentBounds:
    def test_segment_bounds_line(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 5)
        bbox = geo.segment_bounds(1)
        assert bbox == (0.0, 0.0, 10.0, 5.0)

    def test_segment_bounds_move(self):
        geo = Geometry()
        geo.move_to(0, 0)
        assert geo.segment_bounds(0) is None

    def test_segment_bounds_arc(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.arc_to(10, 0, i=5, j=0, clockwise=True)
        bbox = geo.segment_bounds(1)
        assert bbox is not None

    def test_segment_bounds_bezier(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 10, c1x=2, c1y=5, c2x=8, c2y=5)
        bbox = geo.segment_bounds(1)
        assert bbox is not None

    def test_segment_bounds_out_of_range(self):
        geo = Geometry()
        assert geo.segment_bounds(0) is None
        assert geo.segment_bounds(99) is None

    def test_segment_bounds_empty(self):
        geo = Geometry()
        assert geo.segment_bounds(0) is None


class TestSegmentsInFrame:
    def test_segments_in_frame_empty(self):
        geo = Geometry()
        assert geo.segments_in_frame(0, 0, 10, 10) == []

    def test_segments_in_frame_selects_line(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(5, 0)
        geo.line_to(10, 5)
        # Segment 1 (bbox (0,0)-(5,0)) only: x range [0,5] ∩ [2,4] ✓
        assert geo.segments_in_frame(2, -0.5, 4, 0.5) == [1]
        # Segment 2 (bbox (5,0)-(10,5)) only: x range [5,10] ∩ [7,11] ✓
        assert geo.segments_in_frame(7, 2, 11, 6) == [2]

    def test_segments_in_frame_skips_move(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        assert 0 not in geo.segments_in_frame(0, 0, 10, 10)

    def test_segments_in_frame_no_match(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        assert geo.segments_in_frame(20, 20, 30, 30) == []


class TestGetPositionsAtDistances:
    def test_empty_geometry(self):
        geo = Geometry()
        assert geo.get_positions_at_distances([5]) == []

    def test_empty_distances(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        assert geo.get_positions_at_distances([]) == []

    def test_single_line(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        # 40mm path. Midpoint at 5mm -> t=0.5 on segment 1, point (5, 0)
        positions = geo.get_positions_at_distances([5])
        assert len(positions) == 1
        assert positions[0][0] == 1  # segment index
        assert positions[0][1] == pytest.approx(0.5)  # t
        assert positions[0][2] == (5.0, 0.0)  # point

    def test_closed_square(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)
        geo.line_to(0, 10)
        geo.close_path()

        total = geo.distance()
        assert total == pytest.approx(40.0)

        # Equidistant tabs like the tab_cmd logic
        count = 4
        spacing = total / count
        targets = [(i + 0.5) * spacing for i in range(count)]
        positions = geo.get_positions_at_distances(targets)

        assert len(positions) == 4
        # Should land in the middle of each side
        assert positions[0] == (1, 0.5, (5.0, 0.0))
        assert positions[1] == (2, 0.5, (10.0, 5.0))
        assert positions[2] == (3, 0.5, (5.0, 10.0))
        assert positions[3] == (4, 0.5, (0.0, 5.0))

    def test_clamp_to_start(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        positions = geo.get_positions_at_distances([-5])
        assert positions[0][2] == (0.0, 0.0)

    def test_clamp_to_end(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        positions = geo.get_positions_at_distances([100])
        assert positions[0][2] == (10.0, 0.0)

    def test_multiple_distances(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)

        total = geo.distance()
        positions = geo.get_positions_at_distances([0, total / 2, total])
        assert len(positions) == 3
        assert positions[0][2] == (0.0, 0.0)

    def test_with_arc(self):
        geo = Geometry()
        geo.move_to(10, 0)
        # Quarter circle CCW: start (10,0), end (0,10), center (0,0), radius 10
        geo.arc_to(0, 10, i=-10, j=0, clockwise=False)

        # distance() includes Move, but get_positions_at_distances skips it.
        # Use a position within the arc segment only.
        arc_len = 10 * math.pi / 2
        positions = geo.get_positions_at_distances([arc_len / 2])
        assert len(positions) == 1
        _, _, pt = positions[0]
        # Midpoint of quarter circle radius 10 around (0,0) at 45 deg
        assert pt[0] == pytest.approx(7.071, rel=1e-3)
        assert pt[1] == pytest.approx(7.071, rel=1e-3)


class TestGetCombinedRect:
    def test_get_combined_rect_empty(self):
        assert get_combined_rect([]) == (0.0, 0.0, 0.0, 0.0)

    def test_get_combined_rect_single(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 10)
        assert get_combined_rect([geo]) == geo.rect()

    def test_get_combined_rect_multiple(self):
        geo1 = Geometry()
        geo1.move_to(0, 0)
        geo1.line_to(10, 10)

        geo2 = Geometry()
        geo2.move_to(20, 20)
        geo2.line_to(30, 30)

        result = get_combined_rect([geo1, geo2])
        assert result == (0.0, 0.0, 30.0, 30.0)


class TestFilterExt:
    def test_filter_with_bezier(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 10, 3, 5, 7, 2)

        filtered = geo.filter({1})
        assert len(filtered) == 1
        cmd = filtered.data[0]
        assert isinstance(cmd, Bezier)
        assert cmd.end[0] == pytest.approx(10.0)
        assert cmd.end[1] == pytest.approx(10.0)
        assert cmd.control1[0] == pytest.approx(3.0)
        assert cmd.control2[1] == pytest.approx(2.0)

from raygeo import Geometry
from raygeo.geo.shape.point import are_points_equal


def test_are_points_equal_exact():
    """Tests exact point equality."""
    p1 = (1.0, 2.0, 3.0)
    p2 = (1.0, 2.0, 3.0)
    assert are_points_equal(p1, p2, 1e-6)


def test_are_points_equal_within_tolerance():
    """Tests point equality within tolerance."""
    p1 = (1.0, 2.0, 3.0)
    p2 = (1.000001, 2.000001, 3.000001)
    assert are_points_equal(p1, p2, 1e-5)


def test_are_points_equal_outside_tolerance():
    """Tests point inequality outside tolerance."""
    p1 = (1.0, 2.0, 3.0)
    p2 = (1.1, 2.1, 3.1)
    assert not are_points_equal(p1, p2, 1e-6)


def test_are_points_equal_partial_difference():
    """Tests point equality with only some coordinates differing."""
    p1 = (1.0, 2.0, 3.0)
    p2 = (1.0, 2.0, 3.1)
    assert not are_points_equal(p1, p2, 1e-6)


def test_cleanup_empty():
    """Tests cleanup on empty geometry."""
    geo = Geometry()
    result = geo.cleanup(tolerance=1e-6)
    assert result.data is None


def test_cleanup_single_line():
    """Tests that single line segment is preserved."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_duplicate_lines():
    """Tests that duplicate line segments are removed."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2
    assert data[0, 0] == Geometry.CMD_TYPE_MOVE
    assert data[1, 0] == Geometry.CMD_TYPE_LINE


def test_cleanup_three_duplicate_lines():
    """Tests that multiple duplicate line segments are removed."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_different_lines():
    """Tests that different line segments are preserved."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 5)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 3


def test_cleanup_duplicate_arcs():
    """Tests that duplicate arc segments are removed."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.arc_to(10, 0, 5, 0, clockwise=True)
    geo.arc_to(10, 0, 5, 0, clockwise=True)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_different_arcs():
    """Tests that different arc segments are preserved."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.arc_to(10, 0, 5, 0, clockwise=True)
    geo.arc_to(10, 0, 5, 0, clockwise=False)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 3


def test_cleanup_duplicate_beziers():
    """Tests that duplicate bezier segments are removed."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.bezier_to(10, 0, c1x=3, c1y=3, c2x=7, c2y=-3)
    geo.bezier_to(10, 0, c1x=3, c1y=3, c2x=7, c2y=-3)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_different_beziers():
    """Tests that different bezier segments are preserved."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.bezier_to(10, 0, c1x=3, c1y=3, c2x=7, c2y=-3)
    geo.bezier_to(10, 0, c1x=3, c1y=3, c2x=7, c2y=-4)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 3


def test_cleanup_mixed_types():
    """Tests handling of mixed segment types (line and arc with same end)."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(5, 0)
    geo.arc_to(10, 0, 2.5, 0, clockwise=True)
    geo.line_to(5, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    # line_to(5,0) and line_to(5,0) are duplicates, so 4 -> 3
    assert len(data) == 3


def test_cleanup_multiple_paths():
    """Tests that move-to resets duplicate tracking between subpaths."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.move_to(20, 0)
    geo.line_to(30, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 4


def test_cleanup_within_tolerance():
    """Tests that segments within tolerance are considered duplicates."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10.000001, 0)
    result = geo.cleanup(tolerance=1e-5)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_outside_tolerance():
    """Tests that segments outside tolerance are not duplicates."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10.01, 0)
    result = geo.cleanup(tolerance=1e-4)
    data = result.data
    assert data is not None
    assert len(data) == 3


def test_cleanup_preserves_z():
    """Tests that Z coordinates are considered in duplicate check."""
    geo = Geometry()
    geo.move_to(0, 0, 0)
    geo.line_to(10, 0, 0)
    geo.line_to(10, 0, 1)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 3


def test_cleanup_complex_path():
    """Tests handling of a complex path with duplicate in the middle."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 10)
    geo.line_to(10, 10)  # duplicate
    geo.line_to(0, 10)
    geo.line_to(0, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 5


def test_cleanup_no_duplicates():
    """Tests that path without duplicates remains unchanged."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 10)
    geo.line_to(0, 10)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 4


def test_cleanup_different_lines_same_start():
    """Tests segments with same start but different endpoints."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(5, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 3


def test_cleanup_moves_only():
    """Tests that move commands are always preserved."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.move_to(10, 0)
    geo.move_to(20, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 3


def test_cleanup_default_tolerance():
    """Tests that default tolerance works correctly."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_zero_tolerance():
    """Tests behavior with zero tolerance."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 0)
    result = geo.cleanup(tolerance=0.0)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_large_tolerance():
    """Tests behavior with large tolerance (treats all same-end as dup)."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10.1, 0)
    result = geo.cleanup(tolerance=1.0)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_vertical_line_duplicates():
    """Tests duplicate detection on vertical lines."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(0, 10)
    geo.line_to(0, 10)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_diagonal_line_duplicates():
    """Tests duplicate detection on diagonal lines."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)
    geo.line_to(10, 10)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_all_duplicates():
    """Tests when all draw segments are duplicates."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 0)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 2


def test_cleanup_arc_ccw_vs_cw():
    """Tests that CCW and CW arcs are not considered duplicates."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.arc_to(10, 0, 5, 0, clockwise=True)
    geo.arc_to(10, 0, 5, 0, clockwise=False)
    result = geo.cleanup(tolerance=1e-6)
    data = result.data
    assert data is not None
    assert len(data) == 3


def test_close_geometry_gaps_functional():
    """Tests closing gaps via Geometry.close_gaps()."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(10, 10)
    geo.line_to(0.000001, 10)
    geo.line_to(0.000002, 0.000003)

    result = geo.close_gaps(tolerance=1e-5)
    data = result.data
    assert data is not None
    assert result is geo
    assert data[0, 1] == 0 and data[0, 2] == 0
    assert data[-1, 1] == 0 and data[-1, 2] == 0

    geo2 = Geometry()
    geo2.move_to(0, 0)
    geo2.line_to(10, 10)
    geo2.move_to(10.000001, 10.000002)
    geo2.line_to(20, 20)

    result2 = geo2.close_gaps(tolerance=1e-5)
    data2 = result2.data
    assert data2 is not None
    assert result2 is geo2
    assert data2[2, Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE
    assert data2[2, 1] == 10 and data2[2, 2] == 10


def test_close_geometry_gaps_respects_tolerance():
    """Tests that the tolerance parameter is correctly used."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 10)
    geo.move_to(10.1, 10.1)
    geo.line_to(20, 20)

    geo1 = geo.copy()
    result1 = geo1.close_gaps(tolerance=0.1)
    assert result1.data is not None
    assert result1.data[2, Geometry.COL_TYPE] == Geometry.CMD_TYPE_MOVE

    geo2 = geo.copy()
    result2 = geo2.close_gaps(tolerance=0.2)
    assert result2.data is not None
    assert result2.data[2, Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE

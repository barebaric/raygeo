import numpy as np

from raygeo import Geometry


def test_flatten_to_points():
    """Tests flattening via Geometry.segments()."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.arc_to(10, 10, i=5, j=-5, clockwise=False)
    geo.bezier_to(5, 15, c1x=2, c1y=5, c2x=8, c2y=10)

    result = geo.segments()

    # Segments returns list of subpaths (one per move command)
    assert len(result) >= 1

    # Each subpath should have points
    for subpath in result:
        assert len(subpath) > 0

    # First point should be (0, 0, 0)
    assert result[0][0] == (0.0, 0.0, 0.0)

    # At least one subpath has a point from the arc/bezier
    total_points = sum(len(s) for s in result)
    assert total_points > 2


def test_flatten_to_points_empty():
    """Tests flattening geometry with empty geometry."""
    geo = Geometry()
    result = geo.segments()
    assert result == []


def test_linearize_geometry():
    """Tests the Geometry.linearize() method."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.arc_to(10, 10, i=10, j=0, clockwise=False)

    result = geo.linearize(tolerance=0.1)

    data = result.data
    assert data is not None

    # Should contain only MOVE and LINE commands
    cmd_types = data[:, Geometry.COL_TYPE]
    assert Geometry.CMD_TYPE_ARC not in cmd_types
    assert Geometry.CMD_TYPE_MOVE in cmd_types
    assert Geometry.CMD_TYPE_LINE in cmd_types

    # The end point should still be (10, 10)
    end_point = data[-1, 1:4]
    np.testing.assert_allclose(end_point, (10.0, 10.0, 0.0), atol=1e-6)


def test_linearize_geometry_empty():
    """Tests Geometry.linearize() with empty geometry."""
    geo = Geometry()
    result = geo.linearize(tolerance=0.1)
    assert result.data is None

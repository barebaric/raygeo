import math
import pytest
import numpy as np

from raygeo.geo import Geometry
from raygeo.geo.algo.fitting import (
    are_points_collinear,
    fit_circle_to_3_points,
    fit_circle_to_points,
    fit_points_recursive,
    fit_points_with_primitives,
    get_polyline_line_deviation,
    get_polyline_arc_deviation,
    project_circle_center_to_bisector,
)


def test_are_collinear():
    # Collinear points (horizontal)
    points = [(0.0, 0.0, 0.0), (5.0, 0.0, 0.0), (10.0, 0.0, 0.0)]
    assert are_points_collinear(points) is True

    # Collinear points (vertical)
    points = [(0.0, 0.0, 0.0), (0.0, 5.0, 0.0), (0.0, 10.0, 0.0)]
    assert are_points_collinear(points) is True

    # Non-collinear points
    points = [(0.0, 0.0, 0.0), (1.0, 1.0, 0.0), (2.0, 2.1, 0.0)]
    assert are_points_collinear(points) is False


def test_fit_circle_3_points_perfect_circle():
    """Test fitting a circle through three points on a perfect circle."""
    center = (2.0, 3.0)
    radius = 5.0
    angles = [0, np.pi / 3, 2 * np.pi / 3]
    points = [
        (
            center[0] + radius * np.cos(theta),
            center[1] + radius * np.sin(theta),
            0.0,
        )
        for theta in angles
    ]
    result = fit_circle_to_3_points(points[0], points[1], points[2])
    assert result is not None

    (xc, yc), r = result
    assert xc == pytest.approx(center[0], abs=1e-6)
    assert yc == pytest.approx(center[1], abs=1e-6)
    assert r == pytest.approx(radius, abs=1e-6)


def test_fit_circle_3_points_collinear_returns_none():
    """Test collinear points return None."""
    p1 = (0.0, 0.0, 0.0)
    p2 = (2.0, 2.0, 0.0)
    p3 = (5.0, 5.0, 0.0)
    assert fit_circle_to_3_points(p1, p2, p3) is None


def test_fit_circle_3_points_2d_points():
    """Test fitting with 2D points (no z coordinate)."""
    p1 = (0.0, 0.0)
    p2 = (0.0, 2.0)
    p3 = (2.0, 0.0)
    result = fit_circle_to_3_points(p1, p2, p3)
    assert result is not None

    (xc, yc), r = result
    assert xc == pytest.approx(1.0, abs=1e-6)
    assert yc == pytest.approx(1.0, abs=1e-6)
    assert r == pytest.approx(np.sqrt(2), abs=1e-6)


def test_fit_circle_3_points_3d_points():
    """Test fitting with 3D points (z coordinate is ignored)."""
    p1 = (0.0, 0.0, 5.0)
    p2 = (0.0, 2.0, 10.0)
    p3 = (2.0, 0.0, -3.0)
    result = fit_circle_to_3_points(p1, p2, p3)
    assert result is not None

    (xc, yc), r = result
    assert xc == pytest.approx(1.0, abs=1e-6)
    assert yc == pytest.approx(1.0, abs=1e-6)
    assert r == pytest.approx(np.sqrt(2), abs=1e-6)


def test_fit_circle_3_points_small_radius():
    """Test fitting a small-radius circle."""
    center = (0.0, 0.0)
    radius = 0.1
    angles = [0, np.pi / 2, np.pi]
    points = [
        (
            center[0] + radius * np.cos(theta),
            center[1] + radius * np.sin(theta),
            0.0,
        )
        for theta in angles
    ]
    result = fit_circle_to_3_points(points[0], points[1], points[2])
    assert result is not None

    (xc, yc), r = result
    assert xc == pytest.approx(center[0], abs=1e-6)
    assert yc == pytest.approx(center[1], abs=1e-6)
    assert r == pytest.approx(radius, abs=1e-6)


def test_fit_circle_3_points_nearly_collinear():
    """Test nearly collinear points (should return None)."""
    p1 = (0.0, 0.0, 0.0)
    p2 = (1.0, 1e-10, 0.0)
    p3 = (2.0, 2e-10, 0.0)
    assert fit_circle_to_3_points(p1, p2, p3) is None


def test_fit_circle_3_points_offset_center():
    """Test fitting with offset center."""
    center = (10.0, -5.0)
    radius = 3.0
    angles = [np.pi / 4, np.pi / 2, 3 * np.pi / 4]
    points = [
        (
            center[0] + radius * np.cos(theta),
            center[1] + radius * np.sin(theta),
            0.0,
        )
        for theta in angles
    ]
    result = fit_circle_to_3_points(points[0], points[1], points[2])
    assert result is not None

    (xc, yc), r = result
    assert xc == pytest.approx(center[0], abs=1e-6)
    assert yc == pytest.approx(center[1], abs=1e-6)
    assert r == pytest.approx(radius, abs=1e-6)


def test_fit_circle_to_points_collinear_returns_none():
    """Test collinear points return None."""
    points = [(0.0, 0.0, 0.0), (2.0, 2.0, 0.0), (5.0, 5.0, 0.0)]
    assert fit_circle_to_points(points) is None


def test_fit_circle_to_points_perfect_circle():
    """Test perfect circle fitting."""
    center = (2.0, 3.0)
    radius = 5.0
    angles = np.linspace(0, 2 * np.pi, 20)
    points = [
        (
            center[0] + radius * np.cos(theta),
            center[1] + radius * np.sin(theta),
            0.0,
        )
        for theta in angles
    ]
    result = fit_circle_to_points(points)
    assert result is not None

    (xc, yc), r, error = result
    assert xc == pytest.approx(center[0], abs=1e-6)
    assert yc == pytest.approx(center[1], abs=1e-6)
    assert r == pytest.approx(radius, abs=1e-6)
    assert error < 1e-6


def test_fit_circle_to_points_noisy_circle():
    """Test circle fitting with noisy points."""
    center = (-1.0, 4.0)
    radius = 3.0
    np.random.seed(42)  # For reproducibility
    angles = np.linspace(0, 2 * np.pi, 30)
    noise = np.random.normal(scale=0.1, size=(len(angles), 2))

    points = [
        (
            center[0] + radius * np.cos(theta) + dx,
            center[1] + radius * np.sin(theta) + dy,
            0.0,
        )
        for (theta, (dx, dy)) in zip(angles, noise)
    ]
    result = fit_circle_to_points(points)
    assert result is not None

    (xc, yc), r, error = result
    assert xc == pytest.approx(center[0], abs=0.15)
    assert yc == pytest.approx(center[1], abs=0.15)
    assert r == pytest.approx(radius, abs=0.15)
    assert error < 0.2


def test_fit_circle_to_points_insufficient_points():
    """Test 1-2 points or duplicates return None."""
    assert fit_circle_to_points([(0.0, 0.0, 0.0)]) is None
    assert fit_circle_to_points([(1.0, 2.0, 0.0), (3.0, 4.0, 0.0)]) is None
    assert (
        fit_circle_to_points(
            [(5.0, 5.0, 0.0), (5.0, 5.0, 0.0), (5.0, 5.0, 0.0)]
        )
        is None
    )


def test_fit_circle_to_points_small_radius():
    """Test small-radius circle fitting."""
    center = (0.0, 0.0)
    radius = 0.1
    angles = np.linspace(0, 2 * np.pi, 10)
    points = [
        (
            center[0] + radius * np.cos(theta),
            center[1] + radius * np.sin(theta),
            0.0,
        )
        for theta in angles
    ]
    result = fit_circle_to_points(points)
    assert result is not None
    (xc, yc), r, error = result
    assert r == pytest.approx(radius, rel=0.01)


def test_fit_circle_to_points_semicircle_accuracy():
    """
    Verify fit_circle() returns correct parameters for a perfect semicircle.
    """
    center = (5.0, 0.0)
    radius = 10.0
    angles = np.linspace(0, np.pi, 20)
    points = [
        (
            center[0] + radius * np.cos(theta),
            center[1] + radius * np.sin(theta),
            0.0,
        )
        for theta in angles
    ]
    result = fit_circle_to_points(points)
    assert result is not None
    (xc, yc), r, error = result
    assert np.isclose(xc, 5.0, atol=0.001)
    assert np.isclose(yc, 0.0, atol=0.001)
    assert np.isclose(r, 10.0, rtol=0.001)
    assert error < 1e-6


def test_project_circle_center_to_bisector_already_on_bisector():
    """Test center already on bisector remains unchanged."""
    p1 = (0.0, 0.0, 0.0)
    p2 = (2.0, 0.0, 0.0)
    center = (1.0, 1.0)
    result = project_circle_center_to_bisector(p1, p2, center)
    assert result == pytest.approx(center, abs=1e-9)


def test_project_circle_center_to_bisector_offset_center():
    """Test center offset from bisector is projected correctly."""
    p1 = (0.0, 0.0, 0.0)
    p2 = (2.0, 0.0, 0.0)
    center = (1.0, 1.0)
    result = project_circle_center_to_bisector(p1, p2, center)
    assert result == pytest.approx((1.0, 1.0), abs=1e-9)


def test_project_circle_center_to_bisector_diagonal_chord():
    """Test projection for diagonal chord."""
    p1 = (0.0, 0.0, 0.0)
    p2 = (2.0, 2.0, 0.0)
    center = (3.0, 0.0)
    result = project_circle_center_to_bisector(p1, p2, center)
    expected = (2.5, -0.5)
    assert result == pytest.approx(expected, abs=1e-9)


def test_project_circle_center_to_bisector_coincident_points():
    """Test coincident points return original center."""
    p1 = (1.0, 1.0, 0.0)
    p2 = (1.0, 1.0, 0.0)
    center = (5.0, 5.0)
    result = project_circle_center_to_bisector(p1, p2, center)
    assert result == pytest.approx(center, abs=1e-9)


def test_project_circle_center_to_bisector_equal_distances():
    """Test result ensures equal distances to both points."""
    p1 = (0.0, 0.0, 0.0)
    p2 = (4.0, 0.0, 0.0)
    center = (1.0, 2.0)
    result = project_circle_center_to_bisector(p1, p2, center)
    dist1 = math.hypot(result[0] - p1[0], result[1] - p1[1])
    dist2 = math.hypot(result[0] - p2[0], result[1] - p2[1])
    assert dist1 == pytest.approx(dist2, abs=1e-9)


def test_project_circle_center_to_bisector_2d_points():
    """Test with 2D points (no z coordinate)."""
    p1 = (0.0, 0.0)
    p2 = (2.0, 0.0)
    center = (1.5, 1.0)
    result = project_circle_center_to_bisector(p1, p2, center)
    dist1 = math.hypot(result[0] - p1[0], result[1] - p1[1])
    dist2 = math.hypot(result[0] - p2[0], result[1] - p2[1])
    assert dist1 == pytest.approx(dist2, abs=1e-9)


def test_get_polyline_arc_deviation_perfect_arc():
    """Test deviation for a perfect 90-degree arc."""
    center = (7.0, 3.0)
    radius = 5.0
    angles = np.linspace(np.pi / 2, np.pi, 10)
    points = [
        (center[0] + radius * np.cos(t), center[1] + radius * np.sin(t), 0.0)
        for t in angles
    ]
    deviation = get_polyline_arc_deviation(points, center, radius)
    assert deviation < 0.05, f"Deviation too large: {deviation}"


def test_get_polyline_arc_deviation_too_large():
    """Test deviation for a coarse 90-degree arc is correctly high."""
    center = (7.0, 3.0)
    radius = 5.0
    angles = np.linspace(np.pi / 2, np.pi, 5)  # Coarse sampling
    points = [
        (center[0] + radius * np.cos(t), center[1] + radius * np.sin(t), 0.0)
        for t in angles
    ]
    deviation = get_polyline_arc_deviation(points, center, radius)
    assert deviation > 0.05, f"Expected larger deviation: {deviation}"


def test_fit_points_with_primitives_single_line():
    """Tests that collinear points form a single line."""
    points = [(0.0, 0.0, 0.0), (5.0, 0.0, 0.0), (10.0, 0.0, 0.0)]
    tolerance = 0.1
    cmds = fit_points_with_primitives(points, tolerance)

    assert len(cmds) == 1
    cmd = cmds[0]
    assert cmd[Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE
    assert np.allclose((cmd[Geometry.COL_X], cmd[Geometry.COL_Y]), (10.0, 0.0))


def test_fit_points_with_primitives_single_arc():
    """Tests that points on a circle form a single arc."""
    center = (0.0, 0.0)
    radius = 10.0
    # 90 degree arc
    angles = np.linspace(0, np.pi / 2, 20)
    points = [
        (
            center[0] + radius * np.cos(t),
            center[1] + radius * np.sin(t),
            0.0,
        )
        for t in angles
    ]
    tolerance = 0.1
    cmds = fit_points_with_primitives(points, tolerance)

    assert len(cmds) == 1
    cmd = cmds[0]
    assert cmd[Geometry.COL_TYPE] == Geometry.CMD_TYPE_ARC
    assert np.allclose((cmd[Geometry.COL_X], cmd[Geometry.COL_Y]), (0.0, 10.0))
    # Center offset from start point (10, 0) is (-10, 0)
    assert np.allclose(
        (cmd[Geometry.COL_I], cmd[Geometry.COL_J]), (-10.0, 0.0)
    )
    # CCW
    assert cmd[Geometry.COL_CW] == 0.0


def test_fit_points_with_primitives_corner_split():
    """Tests that a sharp corner splits into two lines."""
    # Line 1: (0,0) -> (10,0)
    l1 = [(x, 0.0, 0.0) for x in np.linspace(0, 10, 10)]
    # Line 2: (10,0) -> (10,10)
    l2 = [(10.0, y, 0.0) for y in np.linspace(1, 10, 10)]
    points = l1 + l2

    tolerance = 0.1
    cmds = fit_points_with_primitives(points, tolerance)

    assert len(cmds) == 2
    assert cmds[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE
    assert np.allclose(
        (cmds[0][Geometry.COL_X], cmds[0][Geometry.COL_Y]), (10.0, 0.0)
    )
    assert cmds[1][Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE
    assert np.allclose(
        (cmds[1][Geometry.COL_X], cmds[1][Geometry.COL_Y]), (10.0, 10.0)
    )


def test_fit_points_with_primitives_line_arc_mixed():
    """Tests a straight line followed by an arc."""
    # Line segment
    line_pts = [(x, 0.0, 0.0) for x in np.linspace(0, 10, 11)]
    # Arc segment (tangent start at 10,0)
    # Center at (10, 5), radius 5. Start angle -pi/2, end 0
    angles = np.linspace(-np.pi / 2, 0, 11)
    arc_pts = [
        (10.0 + 5.0 * np.cos(t), 5.0 + 5.0 * np.sin(t), 0.0) for t in angles
    ]
    # Remove duplicate point at transition
    points = line_pts + arc_pts[1:]

    tolerance = 0.1
    cmds = fit_points_with_primitives(points, tolerance)

    # Should detect at least one line and one arc.
    # Depending on resolution and tolerance, it might be perfect or
    # slightly split, but we expect basic types.
    assert len(cmds) >= 2
    assert cmds[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE
    assert cmds[-1][Geometry.COL_TYPE] == Geometry.CMD_TYPE_ARC


def test_get_polyline_line_deviation_collinear():
    """Test max deviation for collinear points."""
    points = [(0.0, 0.0, 0.0), (5.0, 0.0, 0.0), (10.0, 0.0, 0.0)]
    max_dist, max_idx = get_polyline_line_deviation(points, 0, 2)
    assert max_dist < 1e-9
    assert max_idx == 0


def test_get_polyline_line_deviation_with_deviation():
    """Test max deviation finds the furthest point."""
    points = [(0.0, 0.0, 0.0), (5.0, 1.0, 0.0), (10.0, 0.0, 0.0)]
    max_dist, max_idx = get_polyline_line_deviation(points, 0, 2)
    assert max_dist == pytest.approx(1.0, abs=1e-6)
    assert max_idx == 1


def test_get_polyline_line_deviation_coincident_endpoints():
    """Test max deviation with coincident endpoints."""
    points = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.0, 0.0)]
    max_dist, max_idx = get_polyline_line_deviation(points, 0, 2)
    assert max_dist == pytest.approx(1.0, abs=1e-6)
    assert max_idx == 1


def test_fit_points_recursive_line():
    """Test recursive fitting produces a line for collinear points."""
    points = [(0.0, 0.0, 0.0), (5.0, 0.0, 0.0), (10.0, 0.0, 0.0)]
    cmds = fit_points_recursive(points, 0.1, 0, 2)
    assert len(cmds) == 1
    assert cmds[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE


def test_fit_points_recursive_arc():
    """Test recursive fitting produces an arc for circular points."""
    center = (0.0, 0.0)
    radius = 10.0
    angles = np.linspace(0, np.pi / 2, 20)
    points = [
        (center[0] + radius * np.cos(t), center[1] + radius * np.sin(t), 0.0)
        for t in angles
    ]
    cmds = fit_points_recursive(points, 0.1, 0, len(points) - 1)
    assert len(cmds) == 1
    assert cmds[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_ARC


def test_fit_points_recursive_split():
    """Test recursive fitting splits at corner."""
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    cmds = fit_points_recursive(points, 0.1, 0, 2)
    assert len(cmds) == 2
    assert cmds[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE
    assert cmds[1][Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE


def test_fit_points_recursive_empty():
    """Test recursive fitting with invalid range."""
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)]
    cmds = fit_points_recursive(points, 0.1, 0, 0)
    assert len(cmds) == 0


def test_fit_points_recursive_single_point():
    """Test recursive fitting with single point."""
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)]
    cmds = fit_points_recursive(points, 0.1, 0, 1)
    assert len(cmds) == 1
    assert cmds[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE


def test_fit_arcs_simple_line():
    """Tests Geometry.fit_arcs() with a simple line geometry."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)

    result = geo.fit_arcs(0.1)

    # Should preserve move and line commands
    assert result.data is not None
    assert len(result.data) == 2
    assert result.data[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_MOVE
    assert result.data[1][Geometry.COL_TYPE] == Geometry.CMD_TYPE_LINE


def test_fit_arcs_with_bezier():
    """Tests Geometry.fit_arcs() with a bezier curve."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.bezier_to(10, 10, c1x=2, c1y=5, c2x=8, c2y=5)

    result = geo.fit_arcs(0.1)

    # Should convert bezier to lines/arcs
    assert result.data is not None
    assert len(result.data) >= 1
    assert result.data[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_MOVE
    assert result.data[1][Geometry.COL_TYPE] in (
        Geometry.CMD_TYPE_LINE,
        Geometry.CMD_TYPE_ARC,
    )


def test_fit_arcs_empty():
    """Tests Geometry.fit_arcs() with empty geometry."""
    geo = Geometry()
    result = geo.fit_arcs(0.1)
    assert result is not None
    assert result.data is None


def test_fit_curves_preserve_bezier():
    """Tests preserving beziers via Geometry.fit_curves()."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.bezier_to(10, 10, c1x=2, c1y=5, c2x=8, c2y=5)

    result = geo.fit_curves(0.1, beziers=True, arcs=True)

    assert result.data is not None
    assert len(result.data) == 2
    assert result.data[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_MOVE
    assert result.data[1][Geometry.COL_TYPE] == Geometry.CMD_TYPE_BEZIER


def test_fit_curves_linearize_bezier():
    """Tests linearizing beziers via Geometry.fit_curves()."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.bezier_to(10, 10, c1x=2, c1y=5, c2x=8, c2y=5)

    result = geo.fit_curves(0.1, beziers=False, arcs=True)

    assert result.data is not None
    assert len(result.data) >= 2
    assert result.data[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_MOVE
    for row in result.data[1:]:
        assert row[Geometry.COL_TYPE] in (
            Geometry.CMD_TYPE_LINE,
            Geometry.CMD_TYPE_ARC,
        )


def test_fit_curves_preserve_arc():
    """Tests preserving arcs via Geometry.fit_curves()."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.arc_to(10, 0, 5, 0, clockwise=True)

    result = geo.fit_curves(0.1, beziers=True, arcs=True)

    assert result.data is not None
    assert len(result.data) == 2
    assert result.data[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_MOVE
    assert result.data[1][Geometry.COL_TYPE] == Geometry.CMD_TYPE_ARC


def test_fit_curves_linearize_arc():
    """Tests linearizing arcs via Geometry.fit_curves()."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.arc_to(10, 0, 5, 0, clockwise=True)

    result = geo.fit_curves(0.1, beziers=False, arcs=False)

    assert result.data is not None
    assert result.data[0][Geometry.COL_TYPE] == Geometry.CMD_TYPE_MOVE
    for row in result.data[1:]:
        assert row[Geometry.COL_TYPE] in (
            Geometry.CMD_TYPE_LINE,
            Geometry.CMD_TYPE_ARC,
        )


def test_fit_curves_mixed_lines_beziers_arcs():
    """Tests mixed geometry via Geometry.fit_curves()."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(5, 0)
    geo.bezier_to(15, 5, c1x=7, c1y=3, c2x=13, c2y=3)
    geo.arc_to(25, 0, 5, 0, clockwise=True)

    result = geo.fit_curves(0.1, beziers=True, arcs=True)

    assert result.data is not None
    types = [r[Geometry.COL_TYPE] for r in result.data]
    assert Geometry.CMD_TYPE_BEZIER in types
    assert Geometry.CMD_TYPE_ARC in types


def test_fit_curves_backwards_compat():
    """Tests Geometry.fit_curves() and Geometry.fit_arcs() agree."""
    geo = Geometry()
    geo.move_to(0, 0)
    geo.bezier_to(10, 10, c1x=2, c1y=5, c2x=8, c2y=5)

    result_old = geo.fit_arcs(0.1)
    result_new = geo.fit_curves(0.1, beziers=False, arcs=True)

    assert result_old.data is not None
    assert result_new.data is not None
    assert len(result_old.data) == len(result_new.data)
    np.testing.assert_array_equal(result_old.data, result_new.data)

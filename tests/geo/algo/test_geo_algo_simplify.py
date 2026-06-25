from raygeo.geo.algo.simplify import simplify_polyline, simplify_polyline_3d


def test_simplify_straight_line():
    """Collinear points on a straight line are removed."""
    points = [(0, 0, 0), (1, 1, 0), (2, 2, 0), (3, 3, 0), (10, 10, 0)]

    result = simplify_polyline_3d(points, tolerance=0.001)
    assert len(result) == 2
    assert result[0] == (0, 0, 0)
    assert result[1] == (10, 10, 0)


def test_simplify_significant_corner():
    """Points forming a corner > tolerance are kept."""
    points = [(0, 0, 0), (5, 5, 0), (10, 0, 0)]

    result = simplify_polyline_3d(points, tolerance=1.0)
    assert len(result) == 3
    assert result[1] == (5, 5, 0)


def test_simplify_insignificant_bump():
    """A small bump within tolerance is removed."""
    points = [(0, 0, 0), (5, 0.1, 0), (10, 0, 0)]

    result = simplify_polyline_3d(points, tolerance=0.5)
    assert len(result) == 2
    assert result[1] == (10, 0, 0)


def test_simplify_zigzag_removal():
    """High frequency zigzag noise within tolerance is removed."""
    points: list[tuple[float, float, float]] = [(0, 0, 0)]
    for x in range(1, 10):
        y = 0.05 if x % 2 else -0.05
        points.append((float(x), y, 0.0))
    points.append((10.0, 0.0, 0.0))

    result = simplify_polyline_3d(points, tolerance=0.1)
    assert len(result) == 2
    assert result[0] == (0, 0, 0)
    assert result[1] == (10, 0, 0)


def test_simplify_duplicate_points():
    """Consecutive duplicate points are removed/handled."""
    points = [(0, 0, 0), (0, 0, 0), (10, 10, 0), (10, 10, 0)]

    result = simplify_polyline_3d(points, tolerance=0.001)
    assert len(result) == 2
    assert result[0] == (0, 0, 0)
    assert result[1] == (10, 10, 0)


def test_simplify_empty():
    """Empty point list is handled gracefully."""
    result = simplify_polyline_3d([], tolerance=0.1)
    assert result == []


def test_simplify_single_segment():
    """A single segment (2 points) is not reduced."""
    points = [(0, 0, 0), (10, 10, 0)]

    result = simplify_polyline_3d(points, tolerance=100.0)
    assert len(result) == 2
    assert result[0] == (0, 0, 0)
    assert result[1] == (10, 10, 0)


def test_simplify_three_points_all_kept():
    """All 3 points are kept when deviation > tolerance."""
    points = [(0, 0, 0), (5, 5, 0), (10, 0, 0)]

    result = simplify_polyline_3d(points, tolerance=0.1)
    assert len(result) == 3
    assert result[0] == (0, 0, 0)
    assert result[1] == (5, 5, 0)
    assert result[2] == (10, 0, 0)


def test_simplify_three_points_middle_removed():
    """Middle point is removed when deviation < tolerance."""
    points = [(0, 0, 0), (5, 0.01, 0), (10, 0, 0)]

    result = simplify_polyline_3d(points, tolerance=0.1)
    assert len(result) == 2
    assert result[0] == (0, 0, 0)
    assert result[1] == (10, 0, 0)


def test_simplify_complex_shape():
    """Simplification on a more complex point sequence."""
    points = [
        (0, 0, 0),
        (1, 0.1, 0),
        (2, -0.1, 0),
        (3, 0.05, 0),
        (4, -0.05, 0),
        (5, 5, 0),
        (6, 5.1, 0),
        (7, 4.9, 0),
        (10, 0, 0),
    ]

    result = simplify_polyline_3d(points, tolerance=0.5)
    assert len(result) == 6
    assert result[0] == (0, 0, 0)
    assert result[1] == (4, -0.05, 0)
    assert result[2] == (5, 5, 0)
    assert result[3] == (6, 5.1, 0)
    assert result[4] == (7, 4.9, 0)
    assert result[5] == (10, 0, 0)


def test_simplify_zero_tolerance():
    """Zero tolerance keeps all points."""
    points = [(0, 0, 0), (5, 5, 0), (10, 0, 0)]

    result = simplify_polyline_3d(points, tolerance=0.0)
    assert len(result) == 3


def test_simplify_negative_tolerance():
    """Negative tolerance is treated as zero."""
    points = [(0, 0, 0), (5, 5, 0), (10, 0, 0)]

    result = simplify_polyline_3d(points, tolerance=-1.0)
    assert len(result) == 3


def test_simplify_large_tolerance():
    """Very large tolerance reduces to endpoints only."""
    points = [(0, 0, 0), (1, 1, 0), (2, 2, 0), (3, 3, 0), (10, 10, 0)]

    result = simplify_polyline_3d(points, tolerance=1000.0)
    assert len(result) == 2
    assert result[0] == (0, 0, 0)
    assert result[1] == (10, 10, 0)


def test_simplify_vertical_line():
    """A vertical line simplifies correctly."""
    points = [(0, 0, 0), (0, 1, 0), (0, 2, 0), (0, 3, 0), (0, 10, 0)]

    result = simplify_polyline_3d(points, tolerance=0.001)
    assert len(result) == 2
    assert result[0] == (0, 0, 0)
    assert result[1] == (0, 10, 0)


def test_simplify_z_preserved():
    """Z of corner kept when corner exceeds tolerance."""
    points = [(0, 0, 1), (5, 5, 99), (10, 0, 2)]

    result = simplify_polyline_3d(points, tolerance=0.5)
    assert len(result) == 3
    assert result[1] == (5, 5, 99)


def test_simplify_xy_bump_drops_bump_z():
    """When a bump is removed in XY, its Z is also removed."""
    points = [(0, 0, 1), (5, 0.1, 99), (10, 0, 2)]

    result = simplify_polyline_3d(points, tolerance=0.5)
    assert len(result) == 2
    assert result[0] == (0, 0, 1)
    assert result[1] == (10, 0, 2)


def test_simplify_three_points_collinear_with_z():
    """Collinear points with varying Z preserve Z values."""
    points = [(0, 0, 10), (1, 1, 20), (2, 2, 30), (3, 3, 40), (10, 10, 50)]

    result = simplify_polyline_3d(points, tolerance=0.001)
    assert len(result) == 2
    assert result[0] == (0, 0, 10)
    assert result[1] == (10, 10, 50)


def test_simplify_two_points_z():
    """2-point polyline (minimal case) preserves Z."""
    points = [(0, 0, 42), (10, 10, -5)]

    result = simplify_polyline_3d(points, tolerance=100.0)
    assert len(result) == 2
    assert result[0] == (0, 0, 42)
    assert result[1] == (10, 10, -5)


def test_simplify_mixed_z():
    """Complex shape with varying Z values."""
    points = [
        (0, 0, 0),
        (1, 0.1, 10),
        (2, -0.1, 20),
        (3, 0.05, 30),
        (4, -0.05, 40),
        (5, 5, 100),
        (6, 5.1, 110),
        (7, 4.9, 120),
        (10, 0, 200),
    ]

    result = simplify_polyline_3d(points, tolerance=0.5)
    assert len(result) == 6
    assert result[0] == (0, 0, 0)
    assert result[1] == (4, -0.05, 40)
    assert result[2] == (5, 5, 100)
    assert result[3] == (6, 5.1, 110)
    assert result[4] == (7, 4.9, 120)
    assert result[5] == (10, 0, 200)


# --- 2D simplify_polyline tests ---


def test_simplify_2d_straight_line():
    """Collinear 2D points on a straight line are removed."""
    points = [(0, 0), (1, 1), (2, 2), (3, 3), (10, 10)]

    result = simplify_polyline(points, tolerance=0.001)
    assert len(result) == 2
    assert result[0] == (0, 0)
    assert result[1] == (10, 10)


def test_simplify_2d_significant_corner():
    """2D points forming a corner > tolerance are kept."""
    points = [(0, 0), (5, 5), (10, 0)]

    result = simplify_polyline(points, tolerance=1.0)
    assert len(result) == 3
    assert result[1] == (5, 5)


def test_simplify_2d_insignificant_bump():
    """A small 2D bump within tolerance is removed."""
    points = [(0, 0), (5, 0.1), (10, 0)]

    result = simplify_polyline(points, tolerance=0.5)
    assert len(result) == 2
    assert result[1] == (10, 0)


def test_simplify_2d_empty():
    """Empty 2D point list is handled gracefully."""
    result = simplify_polyline([], tolerance=0.1)
    assert result == []


def test_simplify_2d_single_segment():
    """A single 2D segment (2 points) is not reduced."""
    points = [(0, 0), (10, 10)]

    result = simplify_polyline(points, tolerance=100.0)
    assert len(result) == 2
    assert result[0] == (0, 0)
    assert result[1] == (10, 10)


def test_simplify_2d_zero_tolerance():
    """Zero tolerance keeps all 2D points."""
    points = [(0, 0), (5, 5), (10, 0)]

    result = simplify_polyline(points, tolerance=0.0)
    assert len(result) == 3


def test_simplify_2d_large_tolerance():
    """Very large tolerance reduces to endpoints only."""
    points = [(0, 0), (1, 1), (2, 2), (3, 3), (10, 10)]

    result = simplify_polyline(points, tolerance=1000.0)
    assert len(result) == 2
    assert result[0] == (0, 0)
    assert result[1] == (10, 10)


def test_simplify_2d_vertical_line():
    """A vertical line simplifies correctly."""
    points = [(0, 0), (0, 1), (0, 2), (0, 3), (0, 10)]

    result = simplify_polyline(points, tolerance=0.001)
    assert len(result) == 2
    assert result[0] == (0, 0)
    assert result[1] == (0, 10)


def test_simplify_2d_three_points_middle_removed():
    """Middle 2D point is removed when deviation < tolerance."""
    points = [(0, 0), (5, 0.01), (10, 0)]

    result = simplify_polyline(points, tolerance=0.1)
    assert len(result) == 2
    assert result[0] == (0, 0)
    assert result[1] == (10, 0)


def test_simplify_2d_zigzag_removal():
    """High frequency 2D zigzag noise within tolerance is removed."""
    points: list[tuple[float, float]] = [(0.0, 0.0)]
    for x in range(1, 10):
        y = 0.05 if x % 2 else -0.05
        points.append((float(x), y))
    points.append((10.0, 0.0))

    result = simplify_polyline(points, tolerance=0.1)
    assert len(result) == 2
    assert result[0] == (0, 0)
    assert result[1] == (10, 0)


def test_simplify_2d_negative_tolerance():
    """Negative tolerance treated as zero."""
    points = [(0, 0), (5, 5), (10, 0)]

    result = simplify_polyline(points, tolerance=-1.0)
    assert len(result) == 3


def test_simplify_2d_complex_shape():
    """Simplification on a complex 2D point sequence."""
    points = [
        (0, 0),
        (1, 0.1),
        (2, -0.1),
        (3, 0.05),
        (4, -0.05),
        (5, 5),
        (6, 5.1),
        (7, 4.9),
        (10, 0),
    ]

    result = simplify_polyline(points, tolerance=0.5)
    assert len(result) == 6
    assert result[0] == (0, 0)
    assert result[1] == (4, -0.05)
    assert result[2] == (5, 5)
    assert result[3] == (6, 5.1)
    assert result[4] == (7, 4.9)
    assert result[5] == (10, 0)


def test_simplify_2d_duplicate_points():
    """Consecutive duplicate 2D points are handled."""
    points = [(0, 0), (0, 0), (10, 10), (10, 10)]

    result = simplify_polyline(points, tolerance=0.001)
    assert len(result) == 2
    assert result[0] == (0, 0)
    assert result[1] == (10, 10)

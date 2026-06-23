from raygeo.geo.algo.trochoid import trochoid_along_3d


def approx_eq(a, b, tol=1e-9):
    return abs(a - b) < tol


def test_trochoid_straight_segment():
    """Basic trochoid along a straight horizontal segment."""
    pts = trochoid_along_3d(
        [(0, 0), (100, 0)],
        diameter=10,
        engagement_angle_deg=90,
        step_over_ratio=0.2,
        min_loop_radius=0.5,
        z=0,
    )
    assert len(pts) >= 10
    for p in pts:
        assert approx_eq(p[2], 0, 1e-12)
    max_y = max(abs(p[1]) for p in pts)
    assert max_y > 0.5, f"expected lateral oscillation, max_y={max_y}"
    # True trochoid may end offset from carrier endpoint (up to ~2*loop_radius)
    assert abs(pts[0][0]) < 5.0
    assert abs(pts[-1][0] - 100) < 5.0


def test_trochoid_empty_carrier():
    """Single-point carrier → empty."""
    assert trochoid_along_3d([(0, 0)], diameter=10) == []


def test_trochoid_two_points_no_length():
    """Same start/end → empty."""
    assert trochoid_along_3d([(0, 0), (0, 0)], diameter=10) == []


def test_trochoid_zero_diameter():
    """Zero diameter → empty."""
    pts = trochoid_along_3d(
        [(0, 0), (50, 0)],
        diameter=0,
    )
    assert pts == []


def test_trochoid_zero_step_over():
    """Zero step_over ratio → empty."""
    pts = trochoid_along_3d(
        [(0, 0), (50, 0)],
        diameter=10,
        step_over_ratio=0.0,
    )
    assert pts == []


def test_trochoid_engagement_effect():
    """Lower engagement angle should produce larger lateral amplitude."""
    carrier = [(0, 0), (50, 0)]
    low = trochoid_along_3d(
        carrier,
        diameter=10,
        engagement_angle_deg=30,
        step_over_ratio=0.2,
    )
    high = trochoid_along_3d(
        carrier,
        diameter=10,
        engagement_angle_deg=150,
        step_over_ratio=0.2,
    )
    max_y_low = max(abs(p[1]) for p in low)
    max_y_high = max(abs(p[1]) for p in high)
    assert max_y_low > max_y_high, (
        f"low engagement should have larger amplitude, "
        f"got low={max_y_low:.2f} high={max_y_high:.2f}"
    )


def test_trochoid_min_loop_radius():
    """min_loop_radius should set a floor on amplitude."""
    pts = trochoid_along_3d(
        [(0, 0), (50, 0)],
        diameter=10,
        engagement_angle_deg=90,
        step_over_ratio=0.2,
        min_loop_radius=5,
    )
    max_y = max(abs(p[1]) for p in pts)
    assert max_y >= 4.9, f"expected >=5 lateral, got {max_y:.2f}"


def test_trochoid_l_shaped():
    """L-shaped carrier with corner.

    With true trochoidal oscillation in both tangent and normal directions,
    the endpoint can differ from the carrier endpoint by up to ~2*loop_radius.
    """
    pts = trochoid_along_3d(
        [(0, 0), (50, 0), (50, 50)],
        diameter=10,
        engagement_angle_deg=90,
        step_over_ratio=0.2,
    )
    assert len(pts) >= 10
    max_dev = 10.0
    assert abs(pts[-1][0] - 50) < max_dev
    assert abs(pts[-1][1] - 50) < max_dev


def test_trochoid_z_passthrough():
    """All points should have the specified Z."""
    pts = trochoid_along_3d(
        [(0, 0), (30, 0)],
        diameter=8,
        engagement_angle_deg=60,
        step_over_ratio=0.25,
        z=-5.0,
    )
    for p in pts:
        assert approx_eq(p[2], -5.0, 1e-12)


def test_trochoid_vertical_segment():
    """Trochoid along a vertical carrier."""
    pts = trochoid_along_3d(
        [(0, 0), (0, 100)],
        diameter=10,
        engagement_angle_deg=90,
        step_over_ratio=0.2,
    )
    assert len(pts) >= 10
    # Should oscillate in X direction (normal to vertical)
    max_x = max(abs(p[0]) for p in pts)
    assert max_x > 0.5, f"expected X oscillation, got {max_x:.2f}"
    # Y should generally increase
    assert pts[-1][1] > pts[0][1]


def test_trochoid_diagonal_segment():
    """Trochoid along a diagonal carrier."""
    pts = trochoid_along_3d(
        [(0, 0), (100, 100)],
        diameter=10,
        engagement_angle_deg=90,
        step_over_ratio=0.2,
    )
    assert len(pts) >= 10
    # True trochoid may end offset from carrier endpoint (up to ~2*loop_radius)
    assert abs(pts[-1][0] - 100) < 5.0
    assert abs(pts[-1][1] - 100) < 5.0

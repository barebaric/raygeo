from raygeo.geo.algo.ramp import RampStyle, generate_ramp_3d


def approx_eq(a, b, tol=1e-9):
    return abs(a - b) < tol


def test_ramp_linear_basic():
    """Basic horizontal linear ramp."""
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(50, 0),
        z_start=0,
        z_end=-5,
        max_ramp_angle_deg=45,
        style=RampStyle.LINEAR,
    )
    assert len(pts) >= 2
    assert approx_eq(pts[0][0], 0)
    assert approx_eq(pts[0][1], 0)
    assert approx_eq(pts[0][2], 0)
    assert approx_eq(pts[-1][0], 50, 0.15)
    assert approx_eq(pts[-1][1], 0)
    assert approx_eq(pts[-1][2], -5)
    # Z monotonic
    for i in range(len(pts) - 1):
        assert pts[i][2] >= pts[i + 1][2] - 1e-9


def test_ramp_linear_steep_extension():
    """Steep ramp should be extended to satisfy max_angle."""
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(50, 0),
        z_start=0,
        z_end=-50,
        max_ramp_angle_deg=15,
        style=RampStyle.LINEAR,
    )
    assert len(pts) >= 2
    # XY extent should be > the direct 50mm (needs ~186.6mm at 15°)
    dx = pts[-1][0] - pts[0][0]
    dy = pts[-1][1] - pts[0][1]
    ext_xy = (dx * dx + dy * dy) ** 0.5
    assert ext_xy > 150, f"expected extended ramp, got {ext_xy:.1f}mm"
    # Z drop should be 50mm
    assert approx_eq(pts[0][2] - pts[-1][2], 50, 0.5)


def test_ramp_zigzag():
    """ZigZag ramp should oscillate laterally."""
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(50, 0),
        z_start=0,
        z_end=-5,
        max_ramp_angle_deg=45,
        style=RampStyle.ZIG_ZAG,
        lateral_amplitude=2.0,
    )
    assert len(pts) >= 2
    max_y = max(abs(p[1]) for p in pts)
    assert max_y > 1.0, f"expected lateral oscillation, got max_y={max_y}"
    assert approx_eq(pts[0][1], 0)
    # Z monotonic
    for i in range(len(pts) - 1):
        assert pts[i][2] >= pts[i + 1][2] - 1e-9


def test_ramp_zigzag_zero_amplitude():
    """Zero lateral amplitude → same as linear."""
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(50, 0),
        z_start=0,
        z_end=-5,
        max_ramp_angle_deg=45,
        style=RampStyle.ZIG_ZAG,
        lateral_amplitude=0,
    )
    for p in pts:
        assert approx_eq(p[1], 0)


def test_ramp_no_descent():
    """z_end > z_start → empty."""
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(50, 0),
        z_start=0,
        z_end=5,
        max_ramp_angle_deg=45,
        style=RampStyle.LINEAR,
    )
    assert pts == []


def test_ramp_no_xy_motion():
    """Same XY → empty (no ramp possible)."""
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(0, 0),
        z_start=0,
        z_end=-5,
        max_ramp_angle_deg=45,
        style=RampStyle.LINEAR,
    )
    assert pts == []


def test_ramp_non_axial():
    """Ramp along a diagonal."""
    pts = generate_ramp_3d(
        start=(10, 20),
        end=(30, 60),
        z_start=0,
        z_end=-10,
        max_ramp_angle_deg=30,
        style=RampStyle.LINEAR,
    )
    assert len(pts) >= 2
    # Points should lie along the line from start to end
    dx = pts[-1][0] - pts[0][0]
    dy = pts[-1][1] - pts[0][1]
    dir_mag = (dx * dx + dy * dy) ** 0.5
    assert dir_mag > 0


def test_ramp_very_shallow():
    """Very shallow angle should not extend ramp significantly."""
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(500, 0),
        z_start=0,
        z_end=-5,
        max_ramp_angle_deg=45,
        style=RampStyle.LINEAR,
    )
    # Angle = atan(5/500) ≈ 0.57°, well under 45°, so no extension
    assert abs(pts[0][0]) < 1
    assert abs(pts[-1][0] - 500) < 1


def test_ramp_zero_max_angle():
    """Zero max angle → effectively infinite extension."""
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(50, 0),
        z_start=0,
        z_end=-5,
        max_ramp_angle_deg=0,
        style=RampStyle.LINEAR,
    )
    assert len(pts) >= 2
    assert approx_eq(pts[0][2] - pts[-1][2], 5, 0.5)

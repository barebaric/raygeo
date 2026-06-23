import math

from raygeo.geo.algo.helix import HelixDirection, generate_helix_3d


def approx_eq(a, b, tol=1e-9):
    return abs(a - b) < tol


def test_helix_cylindrical():
    """Constant-radius helix over 2 revolutions."""
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=-10,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=math.pi / 4,
    )
    assert len(pts) > 2
    # Start
    assert approx_eq(pts[0][0], 10)
    assert approx_eq(pts[0][1], 0)
    assert approx_eq(pts[0][2], 0)
    # End — 2 full revs return to same angle
    assert approx_eq(pts[-1][0], 10)
    assert approx_eq(pts[-1][1], 0, 1e-6)
    assert approx_eq(pts[-1][2], -10)
    # Radius constant throughout
    for x, y, _ in pts:
        r = math.hypot(x, y)
        assert approx_eq(r, 10, 1e-6), f"radius {r} not 10 at ({x}, {y})"
    # Z monotonic decreasing
    for i in range(len(pts) - 1):
        assert pts[i][2] >= pts[i + 1][2] - 1e-9, (
            f"z not decreasing at {i}: {pts[i][2]} -> {pts[i + 1][2]}"
        )


def test_helix_conical_expand():
    """Helix with expanding radius (conical)."""
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=5,
        end_radius=15,
        z_start=0,
        z_end=-10,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    assert len(pts) > 2
    r0 = math.hypot(pts[0][0], pts[0][1])
    rn = math.hypot(pts[-1][0], pts[-1][1])
    assert approx_eq(r0, 5)
    assert approx_eq(rn, 15)
    # Radius should increase monotonically
    for i in range(len(pts) - 1):
        ri = math.hypot(pts[i][0], pts[i][1])
        rj = math.hypot(pts[i + 1][0], pts[i + 1][1])
        assert rj >= ri - 1e-9, f"radius decreased at {i}: {ri} -> {rj}"


def test_helix_conical_reduce():
    """Helix with reducing radius."""
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=15,
        end_radius=5,
        z_start=0,
        z_end=-10,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    r0 = math.hypot(pts[0][0], pts[0][1])
    rn = math.hypot(pts[-1][0], pts[-1][1])
    assert approx_eq(r0, 15)
    assert approx_eq(rn, 5)


def test_helix_cw_direction():
    """CW should go -Y at first quarter, CCW should go +Y."""
    cw = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=-5,
        pitch=5,
        direction=HelixDirection.Cw,
        angular_step=math.pi / 2,
    )
    ccw = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=-5,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=math.pi / 2,
    )
    assert len(cw) == len(ccw)
    # Index 1 is the first quarter-turn (t=0.25)
    assert cw[1][1] < 0, f"CW should go -Y, got y={cw[1][1]}"
    assert ccw[1][1] > 0, f"CCW should go +Y, got y={ccw[1][1]}"


def test_helix_min_revolutions():
    """Short Z drop with min_revolutions forcing more turns."""
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=-1,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
        min_revolutions=3,
    )
    assert len(pts) >= int(3 * 2 * math.pi / 0.5 * 0.8)


def test_helix_no_descent():
    """z_end > z_start → empty."""
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=5,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    assert pts == []


def test_helix_zero_pitch():
    """Zero pitch → empty."""
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=-10,
        pitch=0,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    assert pts == []


def test_helix_non_origin_center():
    """Helix centered at (5, 10)."""
    pts = generate_helix_3d(
        center=(5, 10),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=-10,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    # Check points are centered around (5, 10)
    for x, y, _ in pts:
        r = math.hypot(x - 5, y - 10)
        assert approx_eq(r, 10, 1e-6), f"radius {r} not 10"


def test_helix_angular_step_controls_resolution():
    """Larger angular_step should produce fewer points."""
    coarse = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=-10,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=2.0,
    )
    fine = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=0,
        z_end=-10,
        pitch=5,
        direction=HelixDirection.Ccw,
        angular_step=0.1,
    )
    assert len(coarse) < len(fine)


def test_helix_z_end_exact():
    """Last point Z should exactly equal z_end."""
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=10,
        z_start=10,
        z_end=-30,
        pitch=10,
        direction=HelixDirection.Ccw,
        angular_step=0.2,
    )
    assert approx_eq(pts[0][2], 10)
    assert approx_eq(pts[-1][2], -30)

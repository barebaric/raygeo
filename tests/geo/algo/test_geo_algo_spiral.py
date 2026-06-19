import math

from raygeo.geo.algo.helix import HelixDirection
from raygeo.geo.algo.spiral import generate_spiral


def approx_eq(a, b, tol=1e-9):
    return abs(a - b) < tol


def test_spiral_outward():
    """Outward spiral: radius increases from 5 to 30."""
    pts = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=5,
        end_radius=30,
        revolutions=3,
        direction=HelixDirection.Ccw,
        angular_step=math.pi / 4,
    )
    assert len(pts) > 2
    r0 = math.hypot(pts[0][0], pts[0][1])
    rn = math.hypot(pts[-1][0], pts[-1][1])
    assert approx_eq(r0, 5)
    assert approx_eq(rn, 30)
    for i in range(len(pts) - 1):
        ri = math.hypot(pts[i][0], pts[i][1])
        rj = math.hypot(pts[i + 1][0], pts[i + 1][1])
        assert rj >= ri - 1e-9, f"radius decreased at {i}: {ri} -> {rj}"


def test_spiral_inward():
    """Inward spiral: radius decreases from 30 to 5."""
    pts = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=30,
        end_radius=5,
        revolutions=3,
        direction=HelixDirection.Ccw,
        angular_step=math.pi / 4,
    )
    r0 = math.hypot(pts[0][0], pts[0][1])
    rn = math.hypot(pts[-1][0], pts[-1][1])
    assert approx_eq(r0, 30)
    assert approx_eq(rn, 5)
    for i in range(len(pts) - 1):
        ri = math.hypot(pts[i][0], pts[i][1])
        rj = math.hypot(pts[i + 1][0], pts[i + 1][1])
        assert ri >= rj - 1e-9, f"radius increased at {i}: {ri} -> {rj}"


def test_spiral_constant_z():
    """All points should have the same Z."""
    pts = generate_spiral(
        center=(0, 0),
        z=42.5,
        start_radius=5,
        end_radius=30,
        revolutions=2,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    for _, _, z in pts:
        assert approx_eq(z, 42.5), f"z={z} not 42.5"


def test_spiral_cw_vs_ccw():
    """CW and CCW should turn in opposite directions."""
    cw = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=10,
        end_radius=20,
        revolutions=1,
        direction=HelixDirection.Cw,
        angular_step=math.pi / 2,
    )
    ccw = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=10,
        end_radius=20,
        revolutions=1,
        direction=HelixDirection.Ccw,
        angular_step=math.pi / 2,
    )
    assert len(cw) == len(ccw)
    # First quarter-turn: CW -> negative Y, CCW -> positive Y
    assert cw[1][1] < 0, f"CW should go -Y, got y={cw[1][1]}"
    assert ccw[1][1] > 0, f"CCW should go +Y, got y={ccw[1][1]}"


def test_spiral_equal_radii():
    """start_radius == end_radius -> empty."""
    pts = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=10,
        end_radius=10,
        revolutions=3,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    assert pts == []


def test_spiral_zero_revolutions():
    """revolutions <= 0 -> empty."""
    pts = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=5,
        end_radius=30,
        revolutions=0,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    assert pts == []


def test_spiral_zero_angular_step():
    """angular_step <= 0 -> empty."""
    pts = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=5,
        end_radius=30,
        revolutions=3,
        direction=HelixDirection.Ccw,
        angular_step=0,
    )
    assert pts == []


def test_spiral_non_origin_center():
    """Spiral centered at (-10, 20)."""
    pts = generate_spiral(
        center=(-10, 20),
        z=5,
        start_radius=5,
        end_radius=15,
        revolutions=2,
        direction=HelixDirection.Ccw,
        angular_step=0.5,
    )
    for x, y, z in pts:
        r = math.hypot(x + 10, y - 20)
        assert approx_eq(r, 5, 1e-6) or r >= 5 - 1e-9, f"radius {r} too small"
        assert approx_eq(z, 5)


def test_spiral_fractional_revolution():
    """Half-turn spiral should work."""
    pts = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=10,
        end_radius=20,
        revolutions=0.5,
        direction=HelixDirection.Ccw,
        angular_step=0.1,
    )
    assert len(pts) > 2
    r0 = math.hypot(pts[0][0], pts[0][1])
    rn = math.hypot(pts[-1][0], pts[-1][1])
    assert approx_eq(r0, 10)
    assert approx_eq(rn, 20, 1e-6)


def test_spiral_angular_step_controls_resolution():
    """Larger angular_step => fewer points."""
    coarse = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=5,
        end_radius=30,
        revolutions=3,
        direction=HelixDirection.Ccw,
        angular_step=2.0,
    )
    fine = generate_spiral(
        center=(0, 0),
        z=0,
        start_radius=5,
        end_radius=30,
        revolutions=3,
        direction=HelixDirection.Ccw,
        angular_step=0.1,
    )
    assert len(coarse) < len(fine)

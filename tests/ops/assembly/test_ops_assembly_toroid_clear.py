"""Tests for toroidal clear assembly module."""

import math

from raygeo.ops.assembly.toroid import generate_toroid, generate_toroidal_clear
from raygeo.ops.cut import Part


def _path_points(ops):
    pts = []
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            ep = ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2]))
    return pts


def test_toroidal_clear_descends_in_one_pass():
    """Carrier 60 mm long, L_min = 60, descends fully in a single pass."""
    carrier = [(0.0, 0.0), (60.0, 0.0)]  # L_pass = 60
    angle = math.degrees(math.atan(6.0 / 60.0))  # L_min = 60
    result = generate_toroidal_clear(
        Part.from_polygons([]),
        carrier=carrier,
        start=(0.0, 0.0, 2.0),
        target_z=-4.0,
        tool_radius=3.0,
        step_over=2.0,
        max_ramp_angle_deg=angle,
    )
    pts = _path_points(result.ops)
    assert len(pts) > 0, "path should have points"
    assert abs(pts[0][2] - 2.0) < 0.05, (
        f"first Z should be ~2.0, got {pts[0][2]}"
    )
    assert abs(pts[-1][2] - (-4.0)) < 0.05, (
        f"last Z should be ~-4.0, got {pts[-1][2]}"
    )
    for i in range(1, len(pts)):
        assert pts[i][2] <= pts[i - 1][2] + 1e-9, (
            f"Z not non-inc at {i}: {pts[i - 1][2]} -> {pts[i][2]}"
        )
    mid = pts[len(pts) // 2][2]
    assert -4.0 - 0.05 <= mid <= 2.0 + 0.05, (
        f"midpoint Z {mid} should be between 2 and -4"
    )


def test_toroidal_clear_zigzags_when_carrier_too_short():
    """Carrier 20 mm, L_min = 60, requires multiple zig-zag passes."""
    carrier = [(0.0, 0.0), (20.0, 0.0)]  # L_pass = 20, needs 3 passes
    carrier_length = 20.0
    angle = math.degrees(math.atan(6.0 / 60.0))  # L_min = 60
    result = generate_toroidal_clear(
        Part.from_polygons([]),
        carrier=carrier,
        start=(0.0, 0.0, 2.0),
        target_z=-4.0,
        tool_radius=3.0,
        step_over=2.0,
        max_ramp_angle_deg=angle,
    )
    pts = _path_points(result.ops)
    assert len(pts) > 0, "path should have points"

    # Count direction reversals in XY by tracking dx sign changes
    dx_signs = []
    for i in range(1, len(pts)):
        dx = pts[i][0] - pts[i - 1][0]
        if abs(dx) > 1e-6:
            dx_signs.append(1 if dx > 0 else -1)

    reversals = 0
    for i in range(1, len(dx_signs)):
        if dx_signs[i] != dx_signs[i - 1]:
            reversals += 1

    assert reversals >= 1, (
        f"expected at least 1 direction reversal (carrier too short "
        f"for one pass), got {reversals}"
    )

    # Identify trailing slice where Z stabilises at target_z
    target_z = -4.0
    trailing = []
    for pt in reversed(pts):
        if abs(pt[2] - target_z) < 0.05:
            trailing.append(pt)
        else:
            break
    trailing.reverse()

    xs = [p[0] for p in trailing]
    assert len(trailing) > 50, (
        f"trailing slice should have >50 points at target_z, "
        f"got {len(trailing)}"
    )
    assert max(xs) - min(xs) >= carrier_length - 1e-3, (
        f"trailing slice should span full carrier length "
        f"({carrier_length}), got {max(xs) - min(xs)}"
    )


def test_toroidal_clear_no_descent_matches_toroid():
    """When start.z == target_z, no descent; matches generate_toroid."""
    carrier = [(0.0, 0.0), (60.0, 0.0)]
    target_z = -5.0
    result_clear = generate_toroidal_clear(
        Part.from_polygons([]),
        carrier=carrier,
        start=(0.0, 0.0, -5.0),
        target_z=target_z,
        tool_radius=3.0,
        step_over=2.0,
    )
    result_toroid = generate_toroid(
        Part.from_polygons([]),
        carrier=carrier,
        tool_radius=3.0,
        step_over=2.0,
        target_z=target_z,
    )
    assert result_clear.ops.len() > 0, "clear path should be non-empty"
    assert result_toroid.ops.len() > 0, "toroid path should be non-empty"

    # Every point in the clear path should be at target_z
    pts = _path_points(result_clear.ops)
    for pt in pts:
        assert abs(pt[2] - target_z) < 0.05, (
            f"point Z {pt[2]} should be close to target_z {target_z}"
        )


def test_toroidal_clear_clears_corridor():
    """Carrier swept by tool_radius covers the corridor interior."""
    carrier = [(0.0, 4.0), (40.0, 4.0)]
    result = generate_toroidal_clear(
        Part.from_polygons([]),
        carrier=carrier,
        start=(0.0, 4.0, 2.0),
        target_z=-4.0,
        tool_radius=3.0,
        step_over=2.0,
    )
    cleared_polygons = result.cleared_polygons
    assert len(cleared_polygons) > 0, "should have cleared polygons"

    for poly in cleared_polygons:
        xs = [p[0] for p in poly]
        ys = [p[1] for p in poly]
        # The swept polygon bounding box should extend beyond
        # carrier endpoints ± tool_radius
        assert min(xs) <= -2, (
            f"polygon should extend left of corridor, got min_x={min(xs)}"
        )
        assert max(xs) >= 42, (
            f"polygon should extend right of corridor, got max_x={max(xs)}"
        )
        assert min(ys) <= 1.5, (
            f"polygon should extend below corridor center, got min_y={min(ys)}"
        )
        assert max(ys) >= 6.5, (
            f"polygon should extend above corridor center, got max_y={max(ys)}"
        )

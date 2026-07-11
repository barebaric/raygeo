"""Tests for slot assembly module."""

from raygeo.ops.assembly.slot import generate_slot
from raygeo.ops.cut import Part


def _path_points(ops):
    pts = []
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            ep = ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2]))
    return pts


def test_slot_single_pass():
    """Flat carrier produces forward+backward path at constant target_z."""
    carrier = [(0.0, 0.0), (40.0, 0.0)]
    result = generate_slot(
        Part.from_polygons([]),
        carrier=carrier,
        tool_radius=3.0,
        target_z=-3.0,
    )
    pts = _path_points(result.ops)
    assert len(pts) > 0, "path should have points"

    # All points at target_z.
    for pt in pts:
        assert abs(pt[2] - (-3.0)) < 0.05, f"point Z {pt[2]} should be ~-3.0"

    # Path visits both endpoints: forward 0→40, backward 40→0.
    xs = [p[0] for p in pts]
    assert max(xs) >= 39.5, f"path should reach far end, max x = {max(xs)}"
    assert pts[0][0] <= 0.5, (
        f"first point should be near start, got x = {pts[0][0]}"
    )
    assert pts[-1][0] <= 0.5, (
        f"last point should return to start, got x = {pts[-1][0]}"
    )


def test_slot_clears_thin_corridor():
    """Carrier swept by tool_radius covers the corridor interior."""
    carrier = [(0.0, 3.0), (40.0, 3.0)]
    part = Part.from_polygons([])
    generate_slot(
        part,
        carrier=carrier,
        tool_radius=3.0,
        target_z=-3.0,
    )
    cleared_polygons = part.cleared.fragments()
    assert len(cleared_polygons) > 0, "should have cleared polygons"

    for poly in cleared_polygons:
        xs = [p[0] for p in poly]
        ys = [p[1] for p in poly]
        # Swept polygon bounding box extends beyond carrier ± tool_radius.
        assert min(xs) <= -2.9, (
            f"polygon should extend left of carrier, min_x={min(xs)}"
        )
        assert max(xs) >= 40.0 + 2.9, (
            f"polygon should extend right of carrier, max_x={max(xs)}"
        )
        # Swept width at least tool diameter.
        width = max(ys) - min(ys)
        assert width >= 2 * 3.0 - 1e-3, f"swept width {width:.3f} < 6.0 - 1e-3"


def test_slot_multi_point_carrier():
    """Multi-point carrier produces forward+backward through all points."""
    part = Part.from_polygons([])
    carrier = [(0.0, 0.0), (20.0, 2.0), (40.0, 0.0)]
    result = generate_slot(
        part,
        carrier=carrier,
        tool_radius=3.0,
        target_z=-3.0,
    )
    pts = _path_points(result.ops)
    assert len(pts) > 0, "path should have points"

    # All points at target_z.
    for pt in pts:
        assert abs(pt[2] - (-3.0)) < 0.05, f"point Z {pt[2]} should be ~-3.0"

    # Forward pass visits all 3 carrier points in order.
    n = len(carrier)
    for i in range(n):
        assert abs(pts[i][0] - carrier[i][0]) < 0.05, (
            f"forward pt {i}: expected x~{carrier[i][0]}, got {pts[i][0]}"
        )
        assert abs(pts[i][1] - carrier[i][1]) < 0.05, (
            f"forward pt {i}: expected y~{carrier[i][1]}, got {pts[i][1]}"
        )

    # Backward pass visits all 3 carrier points in reverse.
    for i in range(n):
        j = n - 1 - i  # reverse index
        k = n + i  # position in path
        assert abs(pts[k][0] - carrier[j][0]) < 0.05, (
            f"backward pt {i}: expected x~{carrier[j][0]}, got {pts[k][0]}"
        )
        assert abs(pts[k][1] - carrier[j][1]) < 0.05, (
            f"backward pt {i}: expected y~{carrier[j][1]}, got {pts[k][1]}"
        )

    # Endpoints match: start at carrier[0], return to carrier[0].
    assert abs(pts[0][0] - carrier[0][0]) < 0.05
    assert abs(pts[0][1] - carrier[0][1]) < 0.05
    assert abs(pts[-1][0] - carrier[0][0]) < 0.05
    assert abs(pts[-1][1] - carrier[0][1]) < 0.05

    # Swept polygon covers the corridor including caps.
    cleared_polygons = part.cleared.fragments()
    assert len(cleared_polygons) > 0
    for poly in cleared_polygons:
        ys = [p[1] for p in poly]
        width = max(ys) - min(ys)
        assert width >= 2 * 3.0 - 1e-3, f"swept width {width:.3f} < 6.0"

import math
from typing import List, Tuple

import pytest

from raygeo.ops import Ops
from raygeo.ops.types import CommandCategory, CommandType

CLIP_RECT = (0.0, 0.0, 100.0, 100.0)


def make_square_region(
    x: float, y: float, w: float, h: float
) -> List[Tuple[float, float]]:
    return [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]


# --- clip_ops_to_regions: lines ---


def test_clip_ops_lines_empty_ops():
    ops = Ops()
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    assert ops.len() == 0


def test_clip_ops_lines_empty_regions():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.clip_ops_to_regions([])
    assert len(ops) == 2


def test_clip_ops_lines_small_regions_skipped():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.clip_ops_to_regions([[(0.0, 0.0), (1.0, 1.0)]])
    assert len(ops) == 2


def test_clip_ops_lines_fully_inside():
    ops = Ops()
    ops.move_to(2, 5)
    ops.line_to(8, 5)
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    segs = list(ops.segment_indices())
    assert len(segs) == 1


def test_clip_ops_lines_fully_outside():
    ops = Ops()
    ops.move_to(20, 20)
    ops.line_to(30, 30)
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    assert len(list(ops.segment_indices())) == 0


def test_clip_ops_lines_partially_clipped():
    ops = Ops()
    ops.move_to(0, 5)
    ops.line_to(20, 5)
    regions = [make_square_region(5, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    segs = list(ops.segment_indices())
    assert len(segs) == 1
    seg = segs[0]
    assert ops.endpoint(seg[0]) is not None
    assert ops.endpoint(seg[-1]) is not None
    assert ops.endpoint(seg[0])[0] >= 5.0
    assert ops.endpoint(seg[-1])[0] <= 15.0


def test_clip_ops_lines_multiple_regions():
    ops = Ops()
    ops.move_to(0, 5)
    ops.line_to(20, 5)
    regions = [
        make_square_region(0, 0, 5, 10),
        make_square_region(15, 0, 5, 10),
    ]
    ops.clip_ops_to_regions(regions)
    segs = list(ops.segment_indices())
    assert len(segs) == 2


# --- clip_ops_to_regions: arcs ---


def test_clip_ops_arcs_fully_inside_preserved():
    ops = Ops()
    ops.move_to(4, 5)
    ops.arc_to(6, 5, 1, 0, clockwise=True)
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    arc_indices = ops.indices_of(CommandType.ARC_TO)
    assert len(arc_indices) == 1
    assert ops.endpoint(arc_indices[0]) == pytest.approx((6, 5, 0), abs=1e-6)


def test_clip_ops_arcs_fully_outside_removed():
    ops = Ops()
    ops.move_to(50, 50)
    ops.arc_to(52, 50, 1, 0, clockwise=True)
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    assert len(list(ops.segment_indices())) == 0


def test_clip_ops_arcs_partially_outside_refitted():
    ops = Ops()
    ops.move_to(1, 5)
    ops.arc_to(9, 5, 4, 0, clockwise=True)
    regions = [make_square_region(3, 0, 4, 10)]
    ops.clip_ops_to_regions(regions)
    arc_indices = ops.indices_of(CommandType.ARC_TO)
    assert len(arc_indices) >= 1
    for seg_indices in ops.segment_indices():
        for i in seg_indices:
            if ops.category(i) == CommandCategory.MOVING:
                assert 3.0 <= ops.endpoint(i)[0] <= 7.0


def test_clip_ops_arcs_mixed_lines_and_arcs_inside():
    ops = Ops()
    ops.move_to(2, 5)
    ops.line_to(4, 5)
    ops.arc_to(6, 5, 1, 0, clockwise=True)
    ops.line_to(8, 5)
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    arc_indices = ops.indices_of(CommandType.ARC_TO)
    assert len(arc_indices) == 1
    assert len(list(ops.segment_indices())) == 1


def test_clip_ops_arcs_rounded_rect_all_corners_preserved():
    r = 0.5
    x, y = 2, 3
    w, h = 6, 4
    ops = Ops()
    ops.move_to(x + r, y)
    ops.line_to(x + w - r, y)
    ops.arc_to(x + w, y + r, 0, r, clockwise=True)
    ops.line_to(x + w, y + h - r)
    ops.arc_to(x + w - r, y + h, -r, 0, clockwise=True)
    ops.line_to(x + r, y + h)
    ops.arc_to(x, y + h - r, 0, -r, clockwise=True)
    ops.line_to(x, y + r)
    ops.arc_to(x + r, y, r, 0, clockwise=True)
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    arc_indices = ops.indices_of(CommandType.ARC_TO)
    assert len(arc_indices) == 4


def test_clip_ops_arcs_state_preserved_after_refit():
    ops = Ops()
    ops.move_to(1, 5)
    ops.set_power(0.8)
    ops.arc_to(9, 5, 4, 0, clockwise=True)
    ops.preload_state()
    regions = [make_square_region(3, 0, 4, 10)]
    ops.clip_ops_to_regions(regions)
    arc_indices = ops.indices_of(CommandType.ARC_TO)
    for idx in arc_indices:
        state = ops.inspect(idx).state
        assert state is not None
        assert abs(state.power - 0.8) < 1e-6


# --- clip_ops_to_regions: leading state commands ---


def test_clip_ops_state_commands_before_first_move_preserved():
    ops = Ops()
    ops.set_power(0.5)
    ops.move_to(2, 5)
    ops.line_to(8, 5)
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    state_indices = ops.indices_of(CommandType.SET_POWER)
    assert len(state_indices) == 1


# --- clip_ops_to_regions: beziers ---


def test_clip_ops_beziers_fully_inside_preserved():
    ops = Ops()
    ops.move_to(3, 5)
    ops.bezier_to((4, 7, 0), (6, 7, 0), (7, 5, 0))
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    bezier_indices = ops.indices_of(CommandType.BEZIER_TO)
    assert len(bezier_indices) == 1
    assert ops.endpoint(bezier_indices[0]) == pytest.approx(
        (7, 5, 0), abs=1e-6
    )


def test_clip_ops_beziers_fully_outside_removed():
    ops = Ops()
    ops.move_to(50, 50)
    ops.bezier_to((51, 53, 0), (53, 53, 0), (54, 50, 0))
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    assert len(list(ops.segment_indices())) == 0


def test_clip_ops_beziers_partially_outside_refitted():
    ops = Ops()
    ops.move_to(1, 5)
    ops.bezier_to((3, 8, 0), (7, 8, 0), (9, 5, 0))
    regions = [make_square_region(3, 0, 4, 10)]
    ops.clip_ops_to_regions(regions)
    segs = list(ops.segment_indices())
    assert len(segs) >= 1
    for seg_indices in segs:
        for i in seg_indices:
            if ops.category(i) == CommandCategory.MOVING:
                assert 3.0 <= ops.endpoint(i)[0] <= 7.0


def test_clip_ops_beziers_state_preserved_after_refit():
    ops = Ops()
    ops.move_to(1, 5)
    ops.set_power(0.8)
    ops.bezier_to((3, 8, 0), (7, 8, 0), (9, 5, 0))
    ops.preload_state()
    regions = [make_square_region(3, 0, 4, 10)]
    ops.clip_ops_to_regions(regions)
    for i in ops.indices_of(CommandType.ARC_TO) + ops.indices_of(
        CommandType.BEZIER_TO
    ):
        state = ops.inspect(i).state
        assert state is not None
        assert abs(state.power - 0.8) < 1e-6


def test_clip_ops_beziers_mixed_lines_and_beziers_inside():
    ops = Ops()
    ops.move_to(2, 5)
    ops.line_to(3, 5)
    ops.bezier_to((4, 7, 0), (6, 7, 0), (7, 5, 0))
    ops.line_to(8, 5)
    regions = [make_square_region(0, 0, 10, 10)]
    ops.clip_ops_to_regions(regions)
    bezier_indices = ops.indices_of(CommandType.BEZIER_TO)
    assert len(bezier_indices) == 1
    assert len(list(ops.segment_indices())) == 1


# --- clip_rect ---


def test_clip_fully_inside():
    ops = Ops()
    ops.move_to(10, 10, -1)
    ops.line_to(90, 90, -1)
    clipped_ops = ops.clip_rect(CLIP_RECT)
    assert clipped_ops.len() == 2
    assert clipped_ops.command_type(0) == CommandType.MOVE_TO
    assert clipped_ops.command_type(1) == CommandType.LINE_TO
    assert clipped_ops.endpoint(1) == pytest.approx((90.0, 90.0, -1.0))


def test_clip_fully_outside():
    ops = Ops()
    ops.move_to(110, 110, 0)
    ops.line_to(120, 120, 0)
    clipped_ops = ops.clip_rect(CLIP_RECT)
    assert len(clipped_ops) == 0


def test_clip_with_arc():
    """Verify clip works on arcs via the new generic linearize interface."""
    ops = Ops()
    ops.move_to(0, 50)
    ops.arc_to(100, 50, i=50, j=0, clockwise=False)  # Semicircle
    clip_rect = (40.0, 0.0, 60.0, 100.0)  # A vertical slice through the middle
    clipped_ops = ops.clip_rect(clip_rect)

    # Check that there are drawing commands left
    cutting_count = sum(
        1 for i in range(clipped_ops.len()) if clipped_ops.is_cutting(i)
    )
    assert cutting_count > 0

    # Check that all remaining points are within the rect bounds
    for i in range(clipped_ops.len()):
        if clipped_ops.category(i) == CommandCategory.MOVING:
            x, y, z = clipped_ops.endpoint(i)
            assert clip_rect[0] <= x <= clip_rect[2]
            assert clip_rect[1] <= y <= clip_rect[3]


# --- clip_rect: scanline ---


def test_clip_scanlinepowercommand_start_outside():
    """Tests clipping a scanline that starts outside and ends inside."""
    ops = Ops()
    ops.move_to(0, 50, 10)
    ops.scan_to(100, 50, 10, bytearray(range(100)))
    clip_rect = (50, 0, 150, 100)
    clipped_ops = ops.clip_rect(clip_rect)

    assert clipped_ops.len() == 2  # MoveTo, ScanLinePowerCommand
    assert clipped_ops.command_type(0) == CommandType.MOVE_TO
    assert clipped_ops.command_type(1) == CommandType.SCAN_LINE

    # 1. Verify it's still a ScanLinePowerCommand (not linearized)
    assert clipped_ops.command_type(1) == CommandType.SCAN_LINE

    # 2. Verify new geometry (starts at the clip boundary)
    assert clipped_ops.endpoint(0) == pytest.approx((50, 50, 10))
    assert clipped_ops.endpoint(1) == pytest.approx((100, 50, 10))

    # 3. Verify power values are sliced correctly (original was 100 values)
    # The clip starts 50% of the way through the line.
    pv = bytes(clipped_ops.scanline_data(1))
    assert len(pv) == 50
    assert pv[0] == 50
    assert pv[-1] == 99


def test_clip_scanlinepowercommand_crossing_with_z_interp():
    """Tests a scanline that crosses the clip rect with Z interpolation."""
    ops = Ops()
    # Line from (-50, 50, 0) to (150, 50, 200) -> total length 200
    ops.move_to(-50, 50, 0)
    ops.scan_to(150, 50, 200, bytearray(range(200)))
    clip_rect = (0, 0, 100, 100)
    clipped_ops = ops.clip_rect(clip_rect)

    assert clipped_ops.len() == 2
    assert clipped_ops.command_type(1) == CommandType.SCAN_LINE

    # The line starts 50 units before x=0 and ends 50 units after x=100.
    # The clipped portion is from x=0 to x=100.
    # t_start = 50 / 200 = 0.25. t_end = 150 / 200 = 0.75
    expected_z_start = 0 + (0.25 * 200)  # 50
    expected_z_end = 0 + (0.75 * 200)  # 150

    assert clipped_ops.endpoint(0) == pytest.approx((0, 50, expected_z_start))
    assert clipped_ops.endpoint(1) == pytest.approx((100, 50, expected_z_end))

    # Power values should be sliced from index 50 to 150.
    expected_len = int(200 * 0.75) - int(200 * 0.25)
    pv = bytes(clipped_ops.scanline_data(1))
    assert len(pv) == expected_len
    assert pv[0] == 50
    assert pv[-1] == 149


def test_clip_scanlinepowercommand_fully_outside():
    """Tests that a fully outside scanline is removed."""
    ops = Ops()
    ops.move_to(200, 50, 10)
    ops.scan_to(300, 50, 10, bytearray(range(100)))
    clip_rect = (0, 0, 100, 100)
    clipped_ops = ops.clip_rect(clip_rect)
    assert len(clipped_ops) == 0


# --- subtract_regions ---


def test_subtract_regions():
    ops = Ops()
    ops.move_to(0, 50, -5)
    ops.line_to(100, 50, 5)
    region = [(40.0, 45.0), (60.0, 45.0), (60.0, 55.0), (40.0, 55.0)]
    ops.subtract_regions([region])
    assert ops.len() == 4
    assert ops.endpoint(1) == pytest.approx((40.0, 50.0, -1.0))
    assert ops.endpoint(3) == pytest.approx((100.0, 50.0, 5.0))


def test_subtract_regions_with_scanline():
    ops = Ops()
    ops.move_to(0, 50, 0)
    ops.scan_to(100, 50, 0, bytearray([100] * 100))
    # Region to cut out of the middle
    region = [(40.0, 40.0), (60.0, 40.0), (60.0, 60.0), (40.0, 60.0)]
    ops.subtract_regions([region])

    # Expected: M(0,50), S(->40,50), M(60,50), S(->100,50)
    assert ops.len() == 4
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == pytest.approx((0, 50, 0))

    assert ops.command_type(1) == CommandType.SCAN_LINE
    assert ops.endpoint(1) == pytest.approx((40, 50, 0))
    assert len(bytes(ops.scanline_data(1))) == 40

    assert ops.command_type(2) == CommandType.MOVE_TO
    assert ops.endpoint(2) == pytest.approx((60, 50, 0))

    assert ops.command_type(3) == CommandType.SCAN_LINE
    assert ops.endpoint(3) == pytest.approx((100, 50, 0))
    assert len(bytes(ops.scanline_data(3))) == 40


# --- clip_to_regions ---


def test_clip_to_regions_basic():
    ops = Ops()
    ops.move_to(0, 50, -5)
    ops.line_to(100, 50, 5)
    region = [(40.0, 45.0), (60.0, 45.0), (60.0, 55.0), (40.0, 55.0)]

    ops.clip_to_regions([region])

    assert ops.len() == 2
    assert ops.endpoint(0) == pytest.approx((40.0, 50.0, -1.0))
    assert ops.endpoint(1) == pytest.approx((60.0, 50.0, 1.0))


def test_clip_to_regions_fully_outside():
    ops = Ops()
    ops.move_to(0, 50, 0)
    ops.line_to(30, 50, 0)
    region = [(40.0, 45.0), (60.0, 45.0), (60.0, 55.0), (40.0, 55.0)]

    ops.clip_to_regions([region])

    assert len(ops) == 0


def test_clip_to_regions_fully_inside():
    ops = Ops()
    ops.move_to(45, 50, 0)
    ops.line_to(55, 50, 0)
    region = [(40.0, 45.0), (60.0, 45.0), (60.0, 55.0), (40.0, 55.0)]

    ops.clip_to_regions([region])

    assert ops.len() == 2
    assert ops.endpoint(1) == pytest.approx((55, 50, 0))


def test_clip_to_regions_multiple_regions():
    ops = Ops()
    ops.move_to(0, 50, 0)
    ops.line_to(100, 50, 0)
    region1 = [(20.0, 45.0), (30.0, 45.0), (30.0, 55.0), (20.0, 55.0)]
    region2 = [(70.0, 45.0), (80.0, 45.0), (80.0, 55.0), (70.0, 55.0)]

    ops.clip_to_regions([region1, region2])

    assert ops.len() == 4
    assert ops.endpoint(0) == pytest.approx((20.0, 50.0, 0.0))
    assert ops.endpoint(1) == pytest.approx((30.0, 50.0, 0.0))
    assert ops.endpoint(2) == pytest.approx((70.0, 50.0, 0.0))
    assert ops.endpoint(3) == pytest.approx((80.0, 50.0, 0.0))


def test_clip_to_regions_with_scanline():
    ops = Ops()
    ops.move_to(0, 50, 0)
    ops.scan_to(100, 50, 0, bytearray([100] * 100))
    region = [(40.0, 40.0), (60.0, 40.0), (60.0, 60.0), (40.0, 60.0)]

    ops.clip_to_regions([region])

    assert ops.len() == 2
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == pytest.approx((40, 50, 0))

    assert ops.command_type(1) == CommandType.SCAN_LINE
    assert ops.endpoint(1) == pytest.approx((60, 50, 0))
    assert len(bytes(ops.scanline_data(1))) == 20


def test_clip_to_regions_empty_regions():
    ops = Ops()
    ops.move_to(0, 50, 0)
    ops.line_to(100, 50, 0)

    original_len = len(ops)
    ops.clip_to_regions([])

    assert len(ops) == original_len


def test_clip_to_regions_preserves_state_commands():
    ops = Ops()
    ops.set_power(0.8)
    ops.move_to(0, 50, 0)
    ops.line_to(100, 50, 0)
    region = [(40.0, 45.0), (60.0, 45.0), (60.0, 55.0), (40.0, 55.0)]

    ops.clip_to_regions([region])

    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.SET_POWER
    assert ops.command_type(1) == CommandType.MOVE_TO
    assert ops.command_type(2) == CommandType.LINE_TO


def create_circle_polygon(cx, cy, radius, num_segments=32):
    points = []
    for i in range(num_segments):
        angle = 2 * math.pi * i / num_segments
        x = cx + radius * math.cos(angle)
        y = cy + radius * math.sin(angle)
        points.append((x, y))
    return points


# --- clip_to_regions: circular regions ---


def test_clip_to_circle_basic():
    ops = Ops()
    ops.move_to(0, 50, -5)
    ops.line_to(100, 50, 5)
    circle = create_circle_polygon(50, 50, 20)

    ops.clip_to_regions([circle])

    assert ops.len() == 2
    assert ops.endpoint(0)[0] == pytest.approx(30.0, abs=0.5)
    assert ops.endpoint(0)[1] == pytest.approx(50.0)
    assert ops.endpoint(1)[0] == pytest.approx(70.0, abs=0.5)
    assert ops.endpoint(1)[1] == pytest.approx(50.0)


def test_clip_to_circle_fully_outside():
    ops = Ops()
    ops.move_to(0, 50, 0)
    ops.line_to(25, 50, 0)
    circle = create_circle_polygon(50, 50, 20)

    ops.clip_to_regions([circle])

    assert len(ops) == 0


def test_clip_to_circle_fully_inside():
    ops = Ops()
    ops.move_to(45, 50, 0)
    ops.line_to(55, 50, 0)
    circle = create_circle_polygon(50, 50, 20)

    ops.clip_to_regions([circle])

    assert ops.len() == 2
    assert ops.endpoint(1) == pytest.approx((55, 50, 0))


def test_clip_to_multiple_circles():
    ops = Ops()
    ops.move_to(0, 50, 0)
    ops.line_to(100, 50, 0)
    circle1 = create_circle_polygon(25, 50, 10)
    circle2 = create_circle_polygon(75, 50, 10)

    ops.clip_to_regions([circle1, circle2])

    assert ops.len() == 4
    assert ops.endpoint(0)[0] == pytest.approx(15.0, abs=0.5)
    assert ops.endpoint(1)[0] == pytest.approx(35.0, abs=0.5)
    assert ops.endpoint(2)[0] == pytest.approx(65.0, abs=0.5)
    assert ops.endpoint(3)[0] == pytest.approx(85.0, abs=0.5)


def test_clip_to_circle_vertical_line():
    ops = Ops()
    ops.move_to(50, 0, 0)
    ops.line_to(50, 100, 0)
    circle = create_circle_polygon(50, 50, 20)

    ops.clip_to_regions([circle])

    assert ops.len() == 2
    assert ops.endpoint(0)[1] == pytest.approx(30.0, abs=0.5)
    assert ops.endpoint(1)[1] == pytest.approx(70.0, abs=0.5)


def test_clip_to_circle_diagonal_line():
    ops = Ops()
    ops.move_to(0, 0, 0)
    ops.line_to(100, 100, 0)
    circle = create_circle_polygon(50, 50, 20)

    ops.clip_to_regions([circle])

    assert ops.len() == 2
    start_x = ops.endpoint(0)[0]
    end_x = ops.endpoint(1)[0]
    assert start_x == pytest.approx(50 - 20 * math.sqrt(2) / 2, abs=0.5)
    assert end_x == pytest.approx(50 + 20 * math.sqrt(2) / 2, abs=0.5)


def test_clip_to_circle_with_scanline():
    ops = Ops()
    ops.move_to(0, 50, 0)
    ops.scan_to(100, 50, 0, bytearray([100] * 100))
    circle = create_circle_polygon(50, 50, 20)

    ops.clip_to_regions([circle])

    assert ops.len() == 2
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0)[0] == pytest.approx(30.0, abs=0.5)

    assert ops.command_type(1) == CommandType.SCAN_LINE
    assert ops.endpoint(1)[0] == pytest.approx(70.0, abs=0.5)
    assert len(bytes(ops.scanline_data(1))) == pytest.approx(40, abs=2)


def test_clip_to_circle_with_arc():
    ops = Ops()
    ops.move_to(30, 30)
    ops.arc_to(70, 30, i=20, j=0, clockwise=True)
    circle = create_circle_polygon(50, 50, 20)

    ops.clip_to_regions([circle])

    assert ops.len() > 0
    cutting_count = sum(1 for i in range(ops.len()) if ops.is_cutting(i))
    assert cutting_count > 0
    for i in range(ops.len()):
        if ops.category(i) == CommandCategory.MOVING:
            x, y, z = ops.endpoint(i)
            dist = math.hypot(x - 50, y - 50)
            assert dist <= 20.5


def test_clip_to_circle_preserves_state_commands():
    ops = Ops()
    ops.set_power(0.8)
    ops.move_to(0, 50, 0)
    ops.line_to(100, 50, 0)
    circle = create_circle_polygon(50, 50, 20)

    ops.clip_to_regions([circle])

    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.SET_POWER
    assert ops.command_type(1) == CommandType.MOVE_TO
    assert ops.command_type(2) == CommandType.LINE_TO


# --- clip_at ---


def test_clip_at_no_hit():
    """Tests that clip_at does nothing if no point is found."""
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    original_len = len(ops)
    # Point is far away from the path
    assert ops.clip_at(100, 100, 1.0) is False
    assert len(ops) == original_len


def test_clip_at_on_line_segment():
    """Tests creating a gap in a simple line segment."""
    ops = Ops()
    ops.move_to(0, 50, 10)
    ops.line_to(100, 50, 20)  # Z should be interpolated

    # Clip near the midpoint
    assert ops.clip_at(50, 50, 10.0) is True

    # Expected:
    # Move(0,50,10), Line(45,50,14.5), Move(55,50,15.5), Line(100,50,20)
    assert ops.len() == 4
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.command_type(2) == CommandType.MOVE_TO
    assert ops.command_type(3) == CommandType.LINE_TO

    # Check the points
    assert ops.endpoint(1) == pytest.approx((45.0, 50.0, 14.5))
    assert ops.endpoint(2) == pytest.approx((55.0, 50.0, 15.5))
    assert ops.endpoint(3) == pytest.approx((100.0, 50.0, 20.0))


def test_clip_at_on_arc_segment():
    """Tests creating a gap in an arc segment."""
    ops = Ops()
    ops.move_to(10, 0)
    # 90 deg CCW arc, radius 10, center (0,0)
    ops.arc_to(0, 10, i=-10, j=0, clockwise=False)

    # Clip near the 45-degree point on the arc
    point_on_arc_x = 10 * math.cos(math.radians(45))
    point_on_arc_y = 10 * math.sin(math.radians(45))
    assert ops.clip_at(point_on_arc_x, point_on_arc_y, 2.0) is True

    # The arc gets linearized by subtract_regions, so we expect a series
    # of LineTo commands with a gap in the middle.
    assert ops.len() > 3
    # Verify there is a MoveTo command somewhere in the middle,
    # indicating a gap
    assert any(
        ops.command_type(i) == CommandType.MOVE_TO for i in range(1, ops.len())
    ), "No MoveToCommand found, indicating no gap was created."


def test_clip_at_start_of_subpath():
    """Tests clipping at the very beginning of a subpath."""
    ops = Ops()
    ops.move_to(0, 50)
    ops.line_to(100, 50)

    # Clip at x=1, width=2. Should clip from 0 to 2.
    assert ops.clip_at(1, 50, 2.0) is True

    # Expected: Move(0,50), Move(2,50), Line(100,50)
    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == pytest.approx((0, 50, 0))
    assert ops.command_type(1) == CommandType.MOVE_TO
    assert ops.endpoint(1) == pytest.approx((2.0, 50.0, 0.0))
    assert ops.endpoint(2) == pytest.approx((100.0, 50.0, 0.0))


def test_clip_at_end_of_subpath():
    """Tests clipping at the very end of a subpath."""
    ops = Ops()
    ops.move_to(0, 50)
    ops.line_to(100, 50)

    # Clip at x=99, width=2. Should clip from 98 to 100.
    assert ops.clip_at(99, 50, 2.0) is True

    # Expected: Move(0,50), Line(98,50), Move(100,50)
    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == pytest.approx((0, 50, 0))
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.endpoint(1) == pytest.approx((98.0, 50.0, 0.0))
    assert ops.command_type(2) == CommandType.MOVE_TO
    assert ops.endpoint(2) == pytest.approx((100.0, 50.0, 0.0))


def test_clip_at_spans_multiple_segments():
    """
    Tests that a clip correctly creates a gap across the boundary of two
    connected LineTo commands.
    """
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(50, 0)  # Segment 1 (index 1)
    ops.line_to(100, 50)  # Segment 2 (index 2)
    ops.line_to(100, 100)  # Segment 3 (index 3)

    # Clip at (50, 0) with a width of 20.
    # This should remove from x=40 on the first line to some point on the
    # second line.
    assert ops.clip_at(50, 0, 20.0) is True

    # Original: M, L, L, L -> 4 commands
    # Expected: M, L(shortened), M(to skip gap), L(shortened), L -> 5+ commands
    assert ops.len() > 4
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.command_type(2) == CommandType.MOVE_TO

    # The first line segment should end before 50
    assert ops.endpoint(1)[0] < 50
    # The new path should start after 50
    assert ops.endpoint(2)[0] > 50

    # Ensure the entire original path after the clip is still present
    assert ops.endpoint(ops.len() - 1) == pytest.approx((100, 100, 0))


def test_clip_at_ignores_state_commands():
    """
    Tests that clip_at correctly handles state commands, ensuring they are
    not part of the geometric subpath.
    """
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(50, 0)  # Subpath 1
    ops.set_power(1.0)
    ops.move_to(60, 0)
    ops.line_to(100, 0)  # Subpath 2

    # Clip the second line segment
    assert ops.clip_at(80, 0, 10.0) is True

    # Path 1 should be unchanged. Path 2 should be clipped.
    assert ops.len() == 7
    # Path 1
    assert ops.endpoint(0) == (0, 0, 0)
    assert ops.endpoint(1) == (50, 0, 0)
    assert ops.command_type(2) == CommandType.SET_POWER
    # Path 2 (clipped)
    assert ops.endpoint(3) == (60, 0, 0)
    assert ops.endpoint(4) == pytest.approx((75, 0, 0))
    assert ops.endpoint(5) == pytest.approx((85, 0, 0))
    assert ops.endpoint(6) == pytest.approx((100, 0, 0))


def test_clip_at_with_state_commands_in_subpath():
    """
    Tests that clip_at correctly handles state commands within a continuous
    subpath. The geometry index and commands index must be properly aligned.
    """
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(50, 0)  # Segment 1
    ops.set_power(0.5)  # State command in the middle of subpath
    ops.line_to(100, 0)  # Segment 2

    # Clip on the second segment (after the state command)
    # The clip point at x=75 is on segment 2, which is at geometry index 2
    # but would be at commands index 3 if not handled correctly.
    assert ops.clip_at(75, 0, 10.0) is True

    # Expected: Move(0,0), Line(50,0), SetPower, Line(70,0), Move(80,0),
    #           Line(100,0)
    assert ops.len() == 6
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == (0, 0, 0)
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.endpoint(1) == pytest.approx((50.0, 0.0, 0.0))
    assert ops.command_type(2) == CommandType.SET_POWER
    assert ops.command_type(3) == CommandType.LINE_TO
    assert ops.endpoint(3) == pytest.approx((70.0, 0.0, 0.0))
    assert ops.command_type(4) == CommandType.MOVE_TO
    assert ops.endpoint(4) == pytest.approx((80.0, 0.0, 0.0))
    assert ops.command_type(5) == CommandType.LINE_TO
    assert ops.endpoint(5) == pytest.approx((100.0, 0.0, 0.0))


def test_clip_at_end_of_segment_with_state_command():
    """
    Tests clipping at the exact endpoint of a segment when there's a state
    command before the next segment.
    """
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(100, 0)  # Segment 1
    ops.set_power(0.8)  # State command
    ops.line_to(200, 0)  # Segment 2

    # Clip at the exact endpoint of segment 1 (x=100)
    assert ops.clip_at(100, 0, 10.0) is True

    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == (0, 0, 0)
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.endpoint(1) == pytest.approx((95.0, 0.0, 0.0))
    assert ops.command_type(2) == CommandType.MOVE_TO
    assert ops.endpoint(2) == pytest.approx((105.0, 0.0, 0.0))
    assert ops.command_type(3) == CommandType.LINE_TO
    assert ops.endpoint(3) == pytest.approx((200.0, 0.0, 0.0))

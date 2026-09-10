import pytest

from raygeo.ops import Ops
from raygeo.ops.transform import is_position_sensitive
from raygeo.ops.transform.drag_knife import DragKnifeSpec
from raygeo.ops.types import CommandType, SectionType


@pytest.fixture
def square_ops():
    ops = Ops()
    ops.set_power(0.8)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(10, 10, 0)
    ops.line_to(30, 10, 0)
    ops.line_to(30, 30, 0)
    ops.line_to(10, 30, 0)
    ops.line_to(10, 10, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    return ops


def command_types(ops):
    return [ops.command_type(i) for i in range(ops.len())]


class TestBasic:
    def test_empty_ops(self):
        ops = Ops()
        ops.apply_drag_knife(5.0, 90.0)
        assert ops.is_empty()

    def test_zero_offset_no_op(self, square_ops):
        orig = square_ops.copy()
        square_ops.apply_drag_knife(0.0, 90.0)
        assert square_ops.len() == orig.len()
        for i in range(orig.len()):
            assert square_ops.endpoint(i) == orig.endpoint(i)

    def test_negative_offset_no_op(self, square_ops):
        orig_len = square_ops.len()
        square_ops.apply_drag_knife(-1.0, 90.0)
        assert square_ops.len() == orig_len

    def test_no_vector_section_no_change(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        orig_len = ops.len()
        ops.apply_drag_knife(5.0, 90.0)
        assert ops.len() == orig_len

    def test_degenerate_contour_passthrough(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        orig_len = ops.len()
        ops.apply_drag_knife(5.0, 90.0)
        assert ops.len() == orig_len


class TestSquareContour:
    def test_command_sequence(self, square_ops):
        square_ops.apply_drag_knife(5.0, 90.0)
        expected = [
            CommandType.SET_POWER,
            CommandType.OPS_SECTION_START,
            CommandType.MOVE_TO,
            CommandType.SET_POWER,
            CommandType.ARC_TO,
            CommandType.LINE_TO,
            CommandType.ARC_TO,
            CommandType.LINE_TO,
            CommandType.ARC_TO,
            CommandType.LINE_TO,
            CommandType.ARC_TO,
            CommandType.LINE_TO,
            CommandType.ARC_TO,
            CommandType.SET_POWER,
            CommandType.OPS_SECTION_END,
        ]
        assert command_types(square_ops) == expected

    def test_arc_in(self, square_ops):
        square_ops.apply_drag_knife(5.0, 90.0)
        assert square_ops.endpoint(2) == pytest.approx((5.0, 10.0, 0.0))
        assert square_ops.power(3) == pytest.approx(0.8)
        assert square_ops.endpoint(4) == pytest.approx((15.0, 10.0, 0.0))

    def test_pivot_path_is_offset_forward(self, square_ops):
        square_ops.apply_drag_knife(5.0, 90.0)
        assert square_ops.endpoint(5) == pytest.approx((35.0, 10.0, 0.0))
        assert square_ops.endpoint(7) == pytest.approx((30.0, 35.0, 0.0))
        assert square_ops.endpoint(9) == pytest.approx((5.0, 30.0, 0.0))
        assert square_ops.endpoint(11) == pytest.approx((10.0, 5.0, 0.0))

    def test_swivel_arcs_pivot_around_corners(self, square_ops):
        square_ops.apply_drag_knife(5.0, 90.0)
        assert square_ops.endpoint(6) == pytest.approx((30.0, 15.0, 0.0))
        assert square_ops.endpoint(8) == pytest.approx((25.0, 30.0, 0.0))
        assert square_ops.endpoint(10) == pytest.approx((10.0, 25.0, 0.0))

    def test_arc_out(self, square_ops):
        square_ops.apply_drag_knife(5.0, 90.0)
        assert square_ops.endpoint(12) == pytest.approx((10.0, 15.0, 0.0))
        last_power = square_ops.len() - 2
        assert square_ops.power(last_power) == 0.0

    def test_swivel_keeps_blade_down(self, square_ops):
        square_ops.apply_drag_knife(5.0, 90.0)
        powers = [
            square_ops.power(i)
            for i in range(square_ops.len())
            if square_ops.command_type(i) == CommandType.SET_POWER
        ]
        assert powers == [0.8, 0.8, 0.0]

    def test_sharp_corners_lift(self, square_ops):
        square_ops.apply_drag_knife(5.0, 45.0)
        types = command_types(square_ops)
        lifts = [
            i
            for i, t in enumerate(types)
            if t == CommandType.SET_POWER and square_ops.power(i) == 0.0
        ]
        assert len(lifts) == 4
        for idx in lifts[:-1]:
            assert types[idx + 1] == CommandType.ARC_TO
            assert types[idx + 2] == CommandType.SET_POWER
            assert square_ops.power(idx + 2) == pytest.approx(0.8)


class TestEdgeCases:
    def test_open_path(self):
        ops = Ops()
        ops.set_power(0.5)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_drag_knife(2.0, 90.0)
        types = command_types(ops)
        assert types.count(CommandType.ARC_TO) == 2
        assert ops.endpoint(2) == pytest.approx((-2.0, 0.0, 0.0))
        assert ops.endpoint(4) == pytest.approx((2.0, 0.0, 0.0))
        assert ops.endpoint(5) == pytest.approx((12.0, 0.0, 0.0))
        assert ops.endpoint(6) == pytest.approx((8.0, 0.0, 0.0))

    def test_multiple_contours(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(50, 50, 0)
        ops.line_to(60, 50, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_drag_knife(2.0, 90.0)
        moves = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.MOVE_TO
        ]
        assert len(moves) == 2
        assert ops.endpoint(moves[0]) == pytest.approx((-2.0, 0.0, 0.0))
        assert ops.endpoint(moves[1]) == pytest.approx((48.0, 50.0, 0.0))

    def test_arc_contour_keeps_arcs(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 0, 0)
        for _ in range(4):
            ops.arc_to(0, 10, -10, 0, False, 0)
            ops.arc_to(-10, 0, 0, -10, False, 0)
            ops.arc_to(0, -10, 10, 0, False, 0)
            ops.arc_to(10, 0, 0, 10, False, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_drag_knife(1.0, 90.0)
        arc_count = sum(
            1
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.ARC_TO
        )
        assert arc_count == 18

    def test_z_height_preserved(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(0, 0, 3.0)
        ops.line_to(10, 0, 3.0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_drag_knife(2.0, 90.0)
        endpoints = [
            ops.endpoint(i)
            for i in range(ops.len())
            if ops.command_type(i)
            in (CommandType.MOVE_TO, CommandType.LINE_TO, CommandType.ARC_TO)
        ]
        assert all(ep[2] == pytest.approx(3.0) for ep in endpoints)

    def test_non_vector_section_untouched(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.move_to(0, 0, 0)
        ops.line_to(5, 5, 0)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(20, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        orig0 = ops.endpoint(0)
        orig1 = ops.endpoint(1)
        ops.apply_drag_knife(5.0, 90.0)
        assert ops.endpoint(0) == orig0
        assert ops.endpoint(1) == orig1


class TestSpec:
    def test_apply_via_apply_transformers(self, square_ops):
        square_ops.apply_transformers([DragKnifeSpec(5.0, 90.0)])
        types = command_types(square_ops)
        assert types.count(CommandType.ARC_TO) == 5

    def test_equality(self):
        assert DragKnifeSpec(1.0, 45.0) == DragKnifeSpec(1.0, 45.0)
        assert DragKnifeSpec(1.0, 45.0) != DragKnifeSpec(2.0, 45.0)

    def test_position_insensitive(self):
        assert is_position_sensitive(DragKnifeSpec(1.0, 45.0)) is False

    def test_readonly_properties(self):
        spec = DragKnifeSpec(1.5, 60.0)
        assert spec.offset_mm == pytest.approx(1.5)
        assert spec.swivel_angle_deg == pytest.approx(60.0)

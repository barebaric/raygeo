import pytest

from raygeo.ops import Ops
from raygeo.ops.axis import Axis
from raygeo.ops.transform import is_position_sensitive
from raygeo.ops.transform.tangential_knife import TangentialKnifeSpec
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


def headings(ops):
    values = []
    for i in range(ops.len()):
        ea = ops.extra_axes(i)
        if ea is not None:
            values.append(ea[Axis.A])
    return values


class TestBasic:
    def test_empty_ops(self):
        ops = Ops()
        ops.apply_tangential_knife(45.0, 1.0, 5.0)
        assert ops.is_empty()

    def test_no_vector_section_no_change(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        orig_len = ops.len()
        ops.apply_tangential_knife(45.0, 1.0, 5.0)
        assert ops.len() == orig_len

    def test_geometry_unchanged(self, square_ops):
        orig = square_ops.copy()
        square_ops.apply_tangential_knife(90.0, 1.0, 5.0)
        orig_moving = [
            (orig.command_type(i), orig.endpoint(i))
            for i in range(orig.len())
            if orig.command_type(i)
            in (
                CommandType.LINE_TO,
                CommandType.ARC_TO,
                CommandType.BEZIER_TO,
            )
        ]
        it = iter(
            (square_ops.command_type(i), square_ops.endpoint(i))
            for i in range(square_ops.len())
        )
        for ct, ep in orig_moving:
            while True:
                ct2, ep2 = next(it)
                if ct2 == ct and ep2 == ep:
                    break


class TestHeadings:
    def test_square_no_lifts_within_tolerance(self, square_ops):
        square_ops.apply_tangential_knife(90.0, 1.0, 5.0)
        assert headings(square_ops) == [
            0.0,
            0.0,
            90.0,
            90.0,
            180.0,
            180.0,
            270.0,
            270.0,
        ]

    def test_move_to_carries_entry_heading(self, square_ops):
        square_ops.apply_tangential_knife(90.0, 1.0, 5.0)
        assert headings(square_ops)[0] == pytest.approx(0.0)

    def test_heading_unwraps_beyond_360(self):
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.line_to(10, 10, 0)
        ops.line_to(0, 10, 0)
        ops.line_to(0, 0, 0)
        ops.line_to(10, -10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_tangential_knife(90.0, 1.0, 5.0)
        assert headings(ops) == [
            0.0,
            0.0,
            90.0,
            90.0,
            180.0,
            180.0,
            270.0,
            270.0,
            315.0,
            315.0,
        ]

    def test_arc_heading_rotation(self):
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 0, 0)
        ops.arc_to(0, 10, -10, 0, False, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_tangential_knife(45.0, 1.0, 5.0)
        assert headings(ops) == [90.0, 180.0]


class TestLifts:
    def test_sharp_corner_lifts(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.line_to(30, 30, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_tangential_knife(45.0, 1.0, 5.0)

        types = command_types(ops)
        assert types == [
            CommandType.SET_POWER,
            CommandType.OPS_SECTION_START,
            CommandType.MOVE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.OPS_SECTION_END,
        ]
        assert ops.endpoint(4) == pytest.approx((30.0, 10.0, 5.0))
        assert ops.endpoint(5) == pytest.approx((30.0, 10.0, 5.0))
        assert ops.endpoint(6) == pytest.approx((30.0, 10.0, 0.0))
        assert ops.endpoint(7) == pytest.approx((30.0, 30.0, 0.0))
        assert headings(ops) == [0.0, 0.0, 0.0, 90.0, 90.0, 90.0]

    def test_no_lift_below_safe_z(self):
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.line_to(30, 30, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_tangential_knife(45.0, 1.0, 0.0)
        assert command_types(ops).count(CommandType.LINE_TO) == 3
        assert headings(ops) == [0.0, 0.0, 90.0, 90.0]

    def test_small_turn_rotates_in_place(self):
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.line_to(20, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_tangential_knife(60.0, 1.0, 5.0)
        types = command_types(ops)
        assert types == [
            CommandType.OPS_SECTION_START,
            CommandType.MOVE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.OPS_SECTION_END,
        ]
        assert ops.endpoint(3) == pytest.approx((10.0, 0.0, 0.0))
        assert headings(ops) == [0.0, 0.0, 45.0, 45.0]


class TestTightArcs:
    def test_tight_arc_traversed_in_air(self):
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.arc_to(10, 1, 0, 0.5, False, 0)
        ops.line_to(20, 1, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_tangential_knife(45.0, 1.0, 5.0)

        types = command_types(ops)
        assert types == [
            CommandType.OPS_SECTION_START,
            CommandType.MOVE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.ARC_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.LINE_TO,
            CommandType.OPS_SECTION_END,
        ]
        assert ops.endpoint(3) == pytest.approx((10.0, 0.0, 5.0))
        assert ops.endpoint(4) == pytest.approx((10.0, 1.0, 5.0))
        assert ops.endpoint(5) == pytest.approx((10.0, 1.0, 0.0))
        assert ops.endpoint(6) == pytest.approx((10.0, 1.0, 5.0))
        assert ops.endpoint(7) == pytest.approx((10.0, 1.0, 5.0))
        assert ops.endpoint(8) == pytest.approx((10.0, 1.0, 0.0))
        assert ops.endpoint(9) == pytest.approx((20.0, 1.0, 0.0))
        assert headings(ops) == [
            0.0,
            0.0,
            0.0,
            180.0,
            180.0,
            180.0,
            0.0,
            0.0,
            0.0,
        ]


class TestSpec:
    def test_apply_via_apply_transformers(self):
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_transformers([TangentialKnifeSpec(45.0, 1.0, 5.0)])
        assert headings(ops) == [0.0, 0.0]

    def test_equality(self):
        assert TangentialKnifeSpec(45.0, 1.0, 5.0) == TangentialKnifeSpec(
            45.0, 1.0, 5.0
        )
        assert TangentialKnifeSpec(45.0, 1.0, 5.0) != TangentialKnifeSpec(
            45.0, 2.0, 5.0
        )

    def test_position_insensitive(self):
        spec = TangentialKnifeSpec(45.0, 1.0, 5.0)
        assert is_position_sensitive(spec) is False

    def test_readonly_properties(self):
        spec = TangentialKnifeSpec(45.0, 1.0, 5.0)
        assert spec.angle_tolerance_deg == pytest.approx(45.0)
        assert spec.radius_tolerance_mm == pytest.approx(1.0)
        assert spec.safe_z == pytest.approx(5.0)

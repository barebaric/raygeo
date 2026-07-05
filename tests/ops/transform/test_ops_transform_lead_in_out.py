import math

import pytest

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


@pytest.fixture
def ops_fixture():
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


class TestBasic:
    def test_empty_ops(self):
        ops = Ops()
        ops.apply_lead_in_out(5.0, 5.0)
        assert ops.is_empty()

    def test_zero_distances_no_op(self, ops_fixture):
        orig = ops_fixture.len()
        ops_fixture.apply_lead_in_out(0.0, 0.0)
        assert ops_fixture.len() == orig

    def test_no_vector_section_no_change(self):
        ops = Ops()
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
        orig = ops.len()
        ops.apply_lead_in_out(5.0, 5.0)
        assert ops.len() == orig


class TestLeadInOut:
    def test_square_contour_both(self, ops_fixture):
        ops_fixture.apply_lead_in_out(5.0, 5.0)

        assert ops_fixture.command_type(2) == CommandType.MOVE_TO
        assert ops_fixture.endpoint(2) == pytest.approx((5.0, 10.0, 0.0))
        assert ops_fixture.command_type(3) == CommandType.SET_POWER
        assert ops_fixture.power(3) == 0
        assert ops_fixture.command_type(4) == CommandType.LINE_TO
        assert ops_fixture.endpoint(4) == pytest.approx((10.0, 10.0, 0.0))
        assert ops_fixture.command_type(5) == CommandType.SET_POWER
        assert ops_fixture.power(5) == 0.8

        lead_out_idx = ops_fixture.len() - 2
        assert ops_fixture.command_type(lead_out_idx) == CommandType.LINE_TO
        assert ops_fixture.endpoint(lead_out_idx) == pytest.approx(
            (10.0, 5.0, 0.0)
        )

    def test_lead_in_only(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_lead_in_out(5.0, 0.0)

        move_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.MOVE_TO
        ]
        assert len(move_indices) == 1
        assert ops.endpoint(move_indices[0]) == pytest.approx((5.0, 10.0, 0.0))
        assert ops.command_type(ops.len() - 2) == CommandType.LINE_TO
        assert ops.endpoint(ops.len() - 2) == pytest.approx((30.0, 10.0, 0.0))

    def test_lead_out_only(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_lead_in_out(0.0, 5.0)

        assert ops.command_type(2) == CommandType.MOVE_TO
        assert ops.endpoint(2) == pytest.approx((10.0, 10.0, 0.0))
        lead_out_idx = ops.len() - 2
        assert ops.command_type(lead_out_idx) == CommandType.LINE_TO
        assert ops.endpoint(lead_out_idx) == pytest.approx((35.0, 10.0, 0.0))

    def test_diagonal_contour(self):
        ops = Ops()
        ops.set_power(0.5)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(0, 0, 0)
        ops.line_to(10, 10, 0)
        ops.line_to(0, 0, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_lead_in_out(5.0, 5.0)

        norm = 1.0 / math.sqrt(2)
        assert ops.command_type(2) == CommandType.MOVE_TO
        assert ops.endpoint(2) == pytest.approx(
            (-5.0 * norm, -5.0 * norm, 0.0)
        )
        lead_out_idx = ops.len() - 2
        assert ops.command_type(lead_out_idx) == CommandType.LINE_TO
        assert ops.endpoint(lead_out_idx) == pytest.approx(
            (-5.0 * norm, -5.0 * norm, 0.0)
        )

    def test_non_vector_section_untouched(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(5, 5, 0)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(20, 10, 0)
        ops.line_to(10, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        orig_ep0 = ops.endpoint(0)
        orig_ep1 = ops.endpoint(1)
        ops.apply_lead_in_out(5.0, 5.0)
        assert ops.endpoint(0) == orig_ep0
        assert ops.endpoint(1) == orig_ep1

    def test_zero_length_first_segment_skips_lead_in(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_lead_in_out(5.0, 5.0)

        move_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.MOVE_TO
        ]
        assert len(move_indices) == 1
        assert ops.endpoint(move_indices[0]) == pytest.approx(
            (10.0, 10.0, 0.0)
        )
        lead_out_idx = ops.len() - 2
        assert ops.command_type(lead_out_idx) == CommandType.LINE_TO
        assert ops.endpoint(lead_out_idx) == pytest.approx((35.0, 10.0, 0.0))

    def test_multiple_contours_in_section(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.line_to(10, 10, 0)
        ops.move_to(50, 50, 0)
        ops.line_to(70, 50, 0)
        ops.line_to(50, 50, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_lead_in_out(5.0, 5.0)

        move_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.MOVE_TO
        ]
        assert len(move_indices) == 2
        assert ops.endpoint(move_indices[0]) == pytest.approx((5.0, 10.0, 0.0))
        assert ops.endpoint(move_indices[1]) == pytest.approx(
            (45.0, 50.0, 0.0)
        )

    def test_with_z_height(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 3.0)
        ops.line_to(30, 10, 3.0)
        ops.line_to(10, 10, 3.0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_lead_in_out(5.0, 5.0)

        assert ops.command_type(2) == CommandType.MOVE_TO
        assert ops.endpoint(2) == pytest.approx((5.0, 10.0, 3.0))
        lead_out_idx = ops.len() - 2
        assert ops.endpoint(lead_out_idx)[2] == pytest.approx(3.0)

    def test_separate_distances(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.line_to(10, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_lead_in_out(3.0, 7.0)

        move_idx = next(
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.MOVE_TO
        )
        assert ops.endpoint(move_idx) == pytest.approx((7.0, 10.0, 0.0))
        lead_out_idx = ops.len() - 2
        assert ops.endpoint(lead_out_idx) == pytest.approx((3.0, 10.0, 0.0))


# ── smoke tests from original test_ops_assembly ──


def test_assembly_apply_lead_in_out():
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp")
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    ops.line_to(0, 10)
    ops.line_to(0, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    original_len = ops.len()
    ops.apply_lead_in_out(5.0, 5.0)
    assert ops.len() > original_len


def test_assembly_lead_in_out_empty_no_op():
    ops = Ops()
    ops.apply_lead_in_out(5.0, 5.0)
    assert ops.is_empty()


def test_lead_in_out_applied_to_arc_contour():
    """Arc-only contour gets lead-in/out (first_cut_idx must match ArcTo)."""
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

    before = ops.len()
    ops.apply_lead_in_out(2.0, 2.0)
    assert ops.len() > before
    arc_count = sum(
        1 for i in range(ops.len())
        if ops.command_type(i) == CommandType.ARC_TO
    )
    assert arc_count == 16, (
        f'Arcs were lost: expected 16 ARC_TO, found {arc_count}'
    )


def test_lead_in_out_applied_to_bezier_contour():
    """Bezier-only contour gets lead-in/out (first_cut_idx must match
    BezierTo)."""
    ops = Ops()
    ops.set_power(0.8)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(0, 0, 0)
    ops.bezier_to((5, 10, 0), (5, 10, 0), (10, 0, 0))
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    before = ops.len()
    ops.apply_lead_in_out(3.0, 3.0)
    assert ops.len() > before
    bezier_count = sum(
        1 for i in range(ops.len())
        if ops.command_type(i) == CommandType.BEZIER_TO
    )
    assert bezier_count == 1, (
        f'Bezier was lost: expected 1 BEZIER_TO, found {bezier_count}'
    )

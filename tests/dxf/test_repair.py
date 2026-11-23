"""Repairing outlines that arrived as loose segments."""

import pytest
from conftest import document, line

from raygeo.dxf import read
from raygeo.geo import Geometry


def _broken_rectangle(gap=0.02):
    """A rectangle exported as four separate lines with mismatched ends.

    This is the shape of the Onshape exports in rayforge issue #668: each
    side is its own entity and no two corners meet exactly. Every side is
    offset so that no pair of endpoints coincides.
    """
    return document(
        [
            line(gap, 0, 100, 0),
            line(100 + gap, gap, 100 + gap, 60),
            line(100, 60 + gap, 0, 60 + gap),
            line(-gap, 60, -gap, 0),
        ]
    )


class TestRepair:
    def test_fragments_become_one_closed_contour(self):
        geometry = read(_broken_rectangle()).layers[0][1]
        assert len(geometry.split_into_contours()) == 4

        repaired, report = geometry.repair_outlines(0.05)

        contours = repaired.split_into_contours()
        assert len(contours) == 1
        assert contours[0].is_closed(1e-6)
        assert report.joins == 3
        assert report.open_remaining == 0

    def test_area_is_preserved(self):
        geometry = read(_broken_rectangle()).layers[0][1]

        repaired, _ = geometry.repair_outlines(0.05)

        assert repaired.area() == pytest.approx(100 * 60, rel=0.01)

    def test_report_is_not_unchanged(self):
        geometry = read(_broken_rectangle()).layers[0][1]

        _, report = geometry.repair_outlines(0.05)

        assert report.is_unchanged() is False

    def test_tolerance_below_the_gap_does_nothing(self):
        geometry = read(_broken_rectangle(gap=1.0)).layers[0][1]
        before = len(geometry.split_into_contours())

        repaired, report = geometry.repair_outlines(0.05)

        assert len(repaired.split_into_contours()) == before
        assert report.joins == 0
        assert report.closed == 0

    def test_larger_tolerance_closes_the_gap(self):
        geometry = read(_broken_rectangle(gap=1.0)).layers[0][1]

        repaired, report = geometry.repair_outlines(2.0)

        assert len(repaired.split_into_contours()) == 1
        assert report.joins > 0

    def test_closed_contours_are_left_alone(self):
        geometry = Geometry()
        geometry.move_to(0, 0)
        geometry.line_to(10, 0)
        geometry.line_to(10, 10)
        geometry.close_path()

        repaired, report = geometry.repair_outlines(0.1)

        assert report.closed == 0
        assert repaired.area() == pytest.approx(50.0)

    def test_duplicates_are_removed(self):
        geometry = Geometry()
        for _ in range(2):
            geometry.move_to(0, 0)
            geometry.line_to(10, 0)
            geometry.line_to(10, 10)
            geometry.close_path()

        repaired, report = geometry.repair_outlines(0.01)

        assert report.duplicates_removed == 1
        assert len(repaired.split_into_contours()) == 1

    def test_zero_tolerance_disables_repair(self):
        geometry = read(_broken_rectangle()).layers[0][1]

        repaired, report = geometry.repair_outlines(0.0)

        assert report.joins == 0
        assert report.closed == 0
        assert report.open_remaining == 4
        assert len(repaired.split_into_contours()) == 4

    def test_empty_geometry(self):
        repaired, report = Geometry().repair_outlines(0.1)

        assert repaired.is_empty()
        assert report.is_unchanged()

    def test_two_panels_repair_independently(self):
        parts = []
        for offset in (0, 200):
            parts += [
                line(offset + 0, 0, offset + 100, 0),
                line(offset + 100.03, 0, offset + 100.03, 60),
                line(offset + 100, 60, offset + 0, 60),
                line(offset + 0, 60.01, offset + 0, 0),
            ]
        geometry = read(document(parts)).layers[0][1]

        repaired, _ = geometry.repair_outlines(0.05)

        contours = repaired.split_into_contours()
        assert len(contours) == 2
        assert all(contour.is_closed(1e-6) for contour in contours)

    def test_existing_points_are_not_moved(self):
        geometry = read(_broken_rectangle()).layers[0][1]
        before = {tuple(round(v, 6) for v in cmd.end) for cmd in geometry.data}

        repaired, _ = geometry.repair_outlines(0.05)
        after = {tuple(round(v, 6) for v in cmd.end) for cmd in repaired.data}

        assert before <= after

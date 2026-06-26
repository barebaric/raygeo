import pytest

from raygeo.geo.shape.rect import (
    do_rects_intersect,
    does_rect_contain_rect,
    is_point_inside_rect,
)


@pytest.fixture
def selection_rect():
    return (10.0, 10.0, 50.0, 50.0)


def test_is_point_inside_rect(selection_rect):
    # Inside
    assert is_point_inside_rect((25, 25), selection_rect)
    # On edge
    assert is_point_inside_rect((10, 25), selection_rect)
    assert is_point_inside_rect((25, 50), selection_rect)
    # Outside
    assert not is_point_inside_rect((5, 25), selection_rect)
    assert not is_point_inside_rect((60, 25), selection_rect)


def test_rect_a_contains_rect_b(selection_rect):
    contained_rect = (20, 20, 40, 40)
    touching_rect = (10, 20, 40, 40)
    intersecting_rect = (40, 40, 60, 60)
    outside_rect = (100, 100, 120, 120)
    assert does_rect_contain_rect(selection_rect, contained_rect)
    assert does_rect_contain_rect(selection_rect, touching_rect)
    assert not does_rect_contain_rect(selection_rect, intersecting_rect)
    assert not does_rect_contain_rect(selection_rect, outside_rect)


class TestDoRectsIntersect:
    def test_overlapping(self):
        assert do_rects_intersect((0, 0, 10, 10), (5, 5, 15, 15)) is True

    def test_non_overlapping(self):
        assert do_rects_intersect((0, 0, 10, 10), (20, 20, 30, 30)) is False

    def test_touching_edge(self):
        assert do_rects_intersect((0, 0, 10, 10), (10, 0, 20, 10)) is True

    def test_touching_corner(self):
        assert do_rects_intersect((0, 0, 10, 10), (10, 10, 20, 20)) is True

    def test_one_inside_other(self):
        r1 = (0, 0, 20, 20)
        r2 = (5, 5, 15, 15)
        assert do_rects_intersect(r1, r2) is True
        assert do_rects_intersect(r2, r1) is True

    def test_identical(self):
        bbox = (0, 0, 10, 10)
        assert do_rects_intersect(bbox, bbox) is True

    def test_negative_coordinates(self):
        assert do_rects_intersect((-10, -10, 0, 0), (-5, -5, 5, 5)) is True

    def test_separated_horizontally(self):
        assert do_rects_intersect((0, 0, 10, 10), (15, 0, 25, 10)) is False

    def test_separated_vertically(self):
        assert do_rects_intersect((0, 0, 10, 10), (0, 15, 10, 25)) is False

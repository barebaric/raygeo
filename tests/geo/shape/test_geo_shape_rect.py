import pytest

from raygeo.geo.shape.rect import (
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

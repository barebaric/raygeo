"""Port of the reference equal-distance constraint tests against the
raygeo bindings."""

import cairo
import pytest
from conftest import _assert_gradient_matches_fd

from raygeo.sketch.constraints.equal_distance import EqualDistanceConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def test_equal_distance_constraint(setup_env):
    reg = setup_env
    # Segment 1: Length 10
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    # Segment 2: Length 4
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)

    c = EqualDistanceConstraint(p1, p2, p3, p4)

    # Error should be 10 - 4 = 6
    assert c.error(reg) == pytest.approx(6.0)
    assert c.user_visible is True


def test_equal_distance_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)

    c = EqualDistanceConstraint(p1, p2, p3, p4, user_visible=False)
    assert c.user_visible is False

    c2 = EqualDistanceConstraint(p1, p2, p3, p4, user_visible=True)
    assert c2.user_visible is True


def test_equal_distance_targets_segment(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)
    p5 = reg.add_point(5, 5)

    c = EqualDistanceConstraint(p1, p2, p3, p4)

    # Matches pair 1 (order independent)
    assert c.targets_segment(p1, p2) is True
    assert c.targets_segment(p2, p1) is True

    # Matches pair 2 (order independent)
    assert c.targets_segment(p3, p4) is True
    assert c.targets_segment(p4, p3) is True

    # No match
    assert c.targets_segment(p1, p3) is False
    assert c.targets_segment(p1, p5) is False


def test_equal_distance_constraint_gradient(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(15, 15)
    p4 = reg.add_point(15, 22)

    constraint = EqualDistanceConstraint(p1, p2, p3, p4)

    _assert_gradient_matches_fd(reg, constraint, [p1, p2, p3, p4])


def test_equal_distance_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    # Segment 1: Length 10
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    # Segment 2: Length 4
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)

    # Create original constraint
    original = EqualDistanceConstraint(p1, p2, p3, p4)

    # Serialize to dict and rebuild through the model layer
    restored = EqualDistanceConstraint.from_dict(original.to_dict())
    assert restored.to_dict() == original.to_dict()


def test_equal_distance_draw(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)

    c = EqualDistanceConstraint(p1, p2, p3, p4)

    ctx = cairo.Context(cairo.ImageSurface(cairo.FORMAT_ARGB32, 200, 200))

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)

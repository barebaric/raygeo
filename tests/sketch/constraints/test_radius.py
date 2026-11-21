"""Port of the reference radius constraint tests against the
raygeo bindings (entity-reference contract)."""

import cairo
import pytest
from conftest import _assert_gradient_matches_fd

from raygeo.sketch.constraints.radius import RadiusConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _arc(reg):
    start = reg.add_point(10, 0)
    end = reg.add_point(0, 10)
    center = reg.add_point(0, 0)
    arc_id = reg.add_arc(start, end, center)
    return arc_id


def _circle(reg):
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(10, 0)
    circle_id = reg.add_circle(center, radius_pt)
    return circle_id


def test_radius_constraint_on_arc(setup_env):
    reg = setup_env
    arc_id = _arc(reg)

    # Current radius is 10. Target is 10. Error = 0.
    c = RadiusConstraint(arc_id, 10.0)
    assert c.error(reg, None) == pytest.approx(0.0)
    assert c.user_visible is True

    # Target is 5. Error = 10 - 5 = 5.
    c2 = RadiusConstraint(arc_id, 5.0)
    assert c2.error(reg, None) == pytest.approx(5.0)


def test_radius_constraint_on_circle(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    c = RadiusConstraint(circle_id, 10.0)
    assert c.error(reg, None) == pytest.approx(0.0)

    c2 = RadiusConstraint(circle_id, 5.0)
    assert c2.error(reg, None) == pytest.approx(5.0)


def test_radius_constraint_user_visible(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    c = RadiusConstraint(circle_id, 10.0, user_visible=False)
    assert c.user_visible is False

    c2 = RadiusConstraint(circle_id, 10.0, user_visible=True)
    assert c2.user_visible is True


def test_radius_entity_id(setup_env):
    reg = setup_env
    arc_id = _arc(reg)

    c = RadiusConstraint(arc_id, 10.0)
    assert c.entity_id == arc_id


def test_radius_constrains_radius_method(setup_env):
    reg = setup_env
    arc_id = _arc(reg)
    circle_id = _circle(reg)

    c = RadiusConstraint(arc_id, 10.0)

    assert c.constrains_radius(reg, arc_id) is True
    assert c.constrains_radius(reg, circle_id) is False
    assert c.constrains_radius(reg, 999) is False


def test_radius_targets_segment(setup_env):
    reg = setup_env
    arc_id = _arc(reg)
    circle_id = _circle(reg)

    c = RadiusConstraint(arc_id, 10.0)

    assert c.targets_segment(0, 0, arc_id) is True
    assert c.targets_segment(0, 0, circle_id) is False
    assert c.targets_segment(0, 0, None) is False


def test_radius_constraint_invalid_entity(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 10)
    line_id = reg.add_line(p1, p2)

    # The reference guard keeps the error at zero for
    # non-radius-bearing entities.
    c = RadiusConstraint(line_id, 5.0)
    assert c.error(reg, None) == pytest.approx(0.0)


def test_radius_constraint_gradient(setup_env):
    reg = setup_env
    arc_id = _arc(reg)

    constraint = RadiusConstraint(arc_id, 12.0)

    # The arc's defining points: start, center.
    _assert_gradient_matches_fd(reg, constraint, [0, 2])


def test_radius_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    original = RadiusConstraint(circle_id, 10.0)
    restored = RadiusConstraint.from_dict(original.to_dict())
    assert original.error(reg, None) == pytest.approx(
        restored.error(reg, None)
    )
    assert original.user_visible == restored.user_visible


def test_radius_constraint_with_expression(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(5, 0)
    circle_id = reg.add_circle(center, radius_pt)

    c = RadiusConstraint(circle_id, 10.0)
    c.expression = "r_val"

    # Current r=5, target=10. Error = 5 - 10 = -5.
    assert c.error(reg, None) == pytest.approx(-5.0)
    assert c.expression == "r_val"


def test_radius_is_hit(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(100, 0)
    circle_id = reg.add_circle(center, radius_pt)

    c = RadiusConstraint(circle_id, 100)

    threshold = 15.0

    def to_screen(pos):
        return pos

    # Label pos is at (radius + 20) along the radius vector: (120, 0).
    label_x, label_y = 120, 0

    assert c.is_hit(label_x, label_y, reg, to_screen, None, threshold) is True
    assert c.is_hit(0, 0, reg, to_screen, None, threshold) is False


def test_radius_draw(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    c = RadiusConstraint(circle_id, 10.0)

    ctx = cairo.Context(cairo.ImageSurface(cairo.FORMAT_ARGB32, 200, 200))

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)

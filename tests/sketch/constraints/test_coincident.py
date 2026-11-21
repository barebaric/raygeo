"""Port of the reference coincident constraint tests against the
raygeo bindings."""

from unittest.mock import MagicMock

import pytest
from conftest import _assert_gradient_matches_fd

from raygeo.sketch.constraints.coincident import CoincidentConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def test_coincident_constraint(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(3, 4)

    c = CoincidentConstraint(p1, p2)
    # Error is a pair (dx, dy)
    assert c.error(reg) == pytest.approx([0.0 - 3.0, 0.0 - 4.0])
    assert c.user_visible is True


def test_coincident_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(3, 4)

    c = CoincidentConstraint(p1, p2, user_visible=False)
    assert c.user_visible is False

    c2 = CoincidentConstraint(p1, p2, user_visible=True)
    assert c2.user_visible is True


# deferred: targets_segment is only bound on dimensional constraints


def test_coincident_constraint_gradient(setup_env):
    reg = setup_env
    p1_id = reg.add_point(1, 2)
    p2_id = reg.add_point(5, 6)

    constraint = CoincidentConstraint(p1_id, p2_id)

    _assert_gradient_matches_fd(reg, constraint, [p1_id, p2_id])


def test_coincident_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(3, 4)

    # Create original constraint
    original = CoincidentConstraint(p1, p2)

    # Serialize to dict and rebuild through the model layer
    restored = CoincidentConstraint.from_dict(original.to_dict())
    assert restored.to_dict() == original.to_dict()


def test_coincident_is_hit(setup_env):
    reg = setup_env
    p1 = reg.add_point(10, 20)
    p2 = reg.add_point(10, 20)
    c = CoincidentConstraint(p1, p2)

    threshold = 15.0

    # Hit the point
    assert c.is_hit(10, 20, reg, lambda pos: pos, None, threshold) is True
    # Miss the point
    assert c.is_hit(30, 20, reg, lambda pos: pos, None, threshold) is False


# deferred: can_apply_to — command/UI layer


def test_coincident_draw(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(3, 4)

    c = CoincidentConstraint(p1, p2)

    ctx = MagicMock()

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)

"""Port of the reference rotational constraint tests against the
raygeo bindings."""

import math

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.rotational import RotationalConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _solver(reg, constraint, pids):
    points = []
    for pid in pids:
        position = reg.get_point(pid)
        assert position is not None
        points.append(PointDef(pid, position[0], position[1], False))
    return Solver(points, [constraint], [])


def test_error_zero_when_satisfied(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    source = reg.add_point(10, 0)
    target = reg.add_point(0, 10)

    c = RotationalConstraint(center, source, target, math.pi / 2)
    assert c.error(reg) == pytest.approx([0.0, 0.0])


def test_error_nonzero_when_violated(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    source = reg.add_point(10, 0)
    target = reg.add_point(5, 5)

    c = RotationalConstraint(center, source, target, math.pi / 2)
    err_x, err_y = c.error(reg)
    assert err_x == pytest.approx(5.0)
    assert err_y == pytest.approx(-5.0)


def test_rotation_about_offset_center(setup_env):
    reg = setup_env
    center = reg.add_point(100, -50)
    source = reg.add_point(110, -50)
    angle = math.radians(30)
    target = reg.add_point(
        100 + 10 * math.cos(angle), -50 + 10 * math.sin(angle)
    )

    c = RotationalConstraint(center, source, target, angle)
    assert c.error(reg) == pytest.approx([0.0, 0.0])


def test_gradient_matches_finite_difference(setup_env):
    reg = setup_env
    center = reg.add_point(3, -2)
    source = reg.add_point(12.0, 4.0)
    target = reg.add_point(-6.0, 9.0)

    c = RotationalConstraint(center, source, target, math.radians(37))

    pids = [center, source, target]
    eps = 1e-7
    jacobian = _solver(reg, c, pids).jacobian()
    for col, pid in enumerate(pids):
        x, y = reg.get_point(pid)
        for axis, px, py in (
            (0, x + eps, y),
            (1, x, y + eps),
        ):
            reg.set_point_position(pid, px, py)
            plus = _solver(reg, c, pids).residuals()
            dx, dy = (x - eps, y) if axis == 0 else (x, y - eps)
            reg.set_point_position(pid, dx, dy)
            minus = _solver(reg, c, pids).residuals()
            reg.set_point_position(pid, x, y)
            for row, value in enumerate(plus):
                finite = (value - minus[row]) / (2.0 * eps)
                analytic = jacobian[row][2 * col + axis]
                assert finite == pytest.approx(analytic, abs=1e-4)


def test_serialization_round_trip(setup_env):
    reg = setup_env
    c = RotationalConstraint(1, 2, 3, math.radians(45), user_visible=False)
    handle = reg.add_constraint_json(c.to_dict())
    data = reg.constraint_dict(handle)
    restored = RotationalConstraint(
        data["center"], data["p1"], data["p2"], data["value"]
    )
    assert restored.center == 1
    assert restored.p1 == 2
    assert restored.p2 == 3
    assert restored.angle_rad == pytest.approx(c.angle_rad)
    assert data["user_visible"] is False


# deferred: expression value update — expressions are resolved
# caller-side in the binding; there is no update_from_context.

# deferred: depends_on_points — reference reporting is not bound on
# the resolved constraint.

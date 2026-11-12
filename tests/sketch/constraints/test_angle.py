"""Port of the reference angle constraint tests against the raygeo
bindings."""

import math

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.angle import AngleConstraint
from raygeo.sketch.constraints.distance import DistanceConstraint
from raygeo.sketch.registry import EntityRegistry

# Screen-space radius of the angle marker arc (reference
# constraints.angle.ARC_RADIUS).
ARC_RADIUS = 35.0


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _solver(reg, constraints, pids):
    points = []
    for pid in pids:
        position = reg.get_point(pid)
        assert position is not None
        points.append(PointDef(pid, position[0], position[1], False))
    return Solver(points, constraints, [])


def _assert_gradient_matches_fd(reg, constraint, pids, tol=1e-4, eps=1e-6):
    """Compares the assembled analytic Jacobian against a central
    finite difference of the residuals (check_grad port)."""
    base = _solver(reg, [constraint], pids).residuals()
    jacobian = _solver(reg, [constraint], pids).jacobian()
    for col, pid in enumerate(pids):
        x, y = reg.get_point(pid)
        for axis, (px, py) in enumerate(((x + eps, y), (x, y + eps))):
            reg.set_point_position(pid, px, py)
            shifted = _solver(reg, [constraint], pids).residuals()
            reg.set_point_position(pid, x, y)
            for row, value in enumerate(shifted):
                finite = (value - base[row]) / (2.0 * eps)
                analytic = jacobian[row][2 * col + axis]
                assert analytic == pytest.approx(finite, abs=tol)


def _chained_lines(reg, n_segments, seg_len=10.0, turn_deg=30.0):
    px, py = 0.0, 0.0
    p_first = reg.add_point(px, py, fixed=True)
    p_prev = p_first
    joints = []
    lines = []
    angle = 0.0
    for _i in range(n_segments):
        angle += turn_deg if _i % 2 == 0 else -turn_deg
        a = math.radians(angle)
        px += seg_len * math.cos(a)
        py += seg_len * math.sin(a)
        p_next = reg.add_point(px, py)
        reg.add_line(p_prev, p_next)
        joints.append(p_next)
        lines.append((p_prev, p_next))
        p_prev = p_next
    return p_first, joints, lines


def _chain_constraints(reg, p_first, joints, lines, turn_deg=30.0):
    constraints: list[object] = [
        DistanceConstraint(p_first, joints[0], 10.0),
    ]
    for i, line in enumerate(lines):
        if i == 0:
            continue
        prev = lines[i - 1]
        constraints.append(DistanceConstraint(joints[i - 1], joints[i], 10.0))
        theta = turn_deg if i % 2 == 0 else -turn_deg
        target = math.degrees(math.pi - math.radians(theta))
        constraints.append(
            AngleConstraint(prev[0], prev[1], line[0], line[1], target)
        )
    return constraints


def _angle_constraints_of(constraints):
    return [c for c in constraints if isinstance(c, AngleConstraint)]


def test_gradient_matches_error_on_chained_lines(setup_env):
    reg = setup_env
    p_first, joints, lines = _chained_lines(reg, 4)
    constraints = _chain_constraints(reg, p_first, joints, lines)

    eps = 1e-6
    for constraint in _angle_constraints_of(constraints):
        # Jacobian columns follow the constraint's own point order
        # (untouched points are not part of the state vector).
        # Jacobian columns follow the constraint's deduplicated point
        # order (untouched points are not part of the state vector).
        pids = list(
            dict.fromkeys(
                [constraint.p1, constraint.p2, constraint.p3, constraint.p4]
            )
        )
        jacobian = _solver(reg, [constraint], pids).jacobian()
        for col, pid in enumerate(pids):
            x, y = reg.get_point(pid)
            for axis, px, py in (
                (0, x + eps, y),
                (1, x, y + eps),
            ):
                reg.set_point_position(pid, px, py)
                plus = _solver(reg, [constraint], pids).residuals()
                dx, dy = (x - eps, y) if axis == 0 else (x, y - eps)
                reg.set_point_position(pid, dx, dy)
                minus = _solver(reg, [constraint], pids).residuals()
                reg.set_point_position(pid, x, y)
                for row, value in enumerate(plus):
                    finite = (value - minus[row]) / (2.0 * eps)
                    analytic = jacobian[row][2 * col + axis]
                    assert analytic == pytest.approx(finite, abs=1e-4)


def test_solver_converges_after_parameter_change(setup_env):
    reg = setup_env
    p_first, joints, lines = _chained_lines(reg, 13)
    constraints = _chain_constraints(reg, p_first, joints, lines)
    pids = [p_first, *joints]

    solver = _solver(reg, constraints, pids)
    assert solver.solve(1e-10, False)
    assert solver.conflicting(1e-3) == []

    # Resolve the solved positions back into the registry, then
    # rebuild the chain with reduced angle targets.
    positions = solver.positions()
    for pid, (x, y) in zip(pids, positions):
        reg.set_point_position(pid, x, y)

    reduced = []
    for constraint in constraints:
        if isinstance(constraint, AngleConstraint):
            reduced.append(
                AngleConstraint(
                    constraint.p1,
                    constraint.p2,
                    constraint.p3,
                    constraint.p4,
                    constraint.value_deg - 3.0,
                )
            )
        elif isinstance(constraint, DistanceConstraint):
            reduced.append(
                DistanceConstraint(
                    constraint.p1, constraint.p2, constraint.value
                )
            )

    solver = _solver(reg, reduced, pids)
    assert solver.solve(1e-10, False)
    assert solver.conflicting(1e-3) == []

    # Resolve the re-solved positions back into the registry, then
    # verify every constraint error.
    positions = solver.positions()
    for pid, (x, y) in zip(pids, positions):
        reg.set_point_position(pid, x, y)

    for constraint in reduced:
        for error in constraint.error(reg):
            assert abs(error) < 1e-4


def test_angle_constraint_90_degrees(setup_env):
    reg = setup_env

    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(0, 10)
    reg.add_line(p3, p4)

    c = AngleConstraint(p3, p4, p1, p2, 90.0)
    assert c.error(reg)[0] == pytest.approx(0.0, abs=1e-6)


def test_angle_constraint_45_degrees(setup_env):
    reg = setup_env

    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(10, 10)
    reg.add_line(p3, p4)

    c = AngleConstraint(p3, p4, p1, p2, 45.0)
    assert c.error(reg)[0] == pytest.approx(0.0, abs=1e-6)


def test_angle_constraint_non_zero_error(setup_env):
    reg = setup_env

    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(0, 10)
    reg.add_line(p3, p4)

    c = AngleConstraint(p3, p4, p1, p2, 45.0)
    error = c.error(reg)[0]
    assert error == pytest.approx(math.pi / 4 * 10, abs=1e-6)


def test_angle_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(0, 10)
    reg.add_line(p3, p4)

    c = AngleConstraint(p1, p2, p3, p4, 90.0, user_visible=False)
    assert c.user_visible() is False

    c2 = AngleConstraint(p1, p2, p3, p4, 90.0, user_visible=True)
    assert c2.user_visible() is True


def test_angle_constraint_with_expression(setup_env):
    reg = setup_env
    # The binding resolves expressions caller-side: the context value
    # 45.0 for "angle" is passed as the resolved value and the
    # expression string is stored alongside (no update_from_context).
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(10, 10)
    reg.add_line(p3, p4)

    c = AngleConstraint(p3, p4, p1, p2, 45.0)
    c.set_expression("angle")

    assert c.error(reg)[0] == pytest.approx(0.0, abs=1e-6)
    assert c.expression() == "angle"
    assert c.is_mirror_compatible() is False


# deferred: invalid entities — the binding resolves entities to
# points, so an invalid entity id cannot be passed.


def test_angle_constraint_serialization_round_trip(setup_env):
    reg = setup_env

    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(0, 10)
    reg.add_line(p3, p4)

    original = AngleConstraint(p1, p2, p3, p4, 90.0)

    serialized = original.to_dict()

    # The dict carries the resolved point ids, from which the
    # constraint is rebuilt (the reference deserializes through
    # entity ids, which the resolved constraint no longer stores).
    restored = AngleConstraint(
        serialized["p1_idx"],
        serialized["p2_idx"],
        serialized["p3_idx"],
        serialized["p4_idx"],
        serialized["value"],
        serialized["e1_far_idx"],
        serialized["e2_far_idx"],
    )

    assert original.error(reg) == pytest.approx(restored.error(reg), abs=1e-6)
    assert original.user_visible() == restored.user_visible()


def test_angle_constraint_serialization_with_expression(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(0, 10)
    reg.add_line(p3, p4)

    # Value is pre-resolved from the context ("angle" -> 90.0).
    original = AngleConstraint(p1, p2, p3, p4, 90.0)
    original.set_expression("angle")

    serialized = original.to_dict()
    restored = AngleConstraint(
        serialized["p1_idx"],
        serialized["p2_idx"],
        serialized["p3_idx"],
        serialized["p4_idx"],
        serialized["value"],
        serialized["e1_far_idx"],
        serialized["e2_far_idx"],
    )
    restored.set_expression(serialized.get("expression"))

    assert original.error(reg) == pytest.approx(restored.error(reg), abs=1e-6)


def test_angle_is_hit(setup_env):
    reg = setup_env

    threshold = 15.0

    l1p1 = reg.add_point(0, 50)
    l1p2 = reg.add_point(100, 50)
    reg.add_line(l1p1, l1p2)

    l2p1 = reg.add_point(50, 0)
    l2p2 = reg.add_point(50, 100)
    reg.add_line(l2p1, l2p2)

    c = AngleConstraint(l2p1, l2p2, l1p1, l1p2, 90.0)

    hit_x = 50 + ARC_RADIUS * math.cos(math.pi)
    hit_y = 50 + ARC_RADIUS * math.sin(math.pi)
    assert c.is_hit(hit_x, hit_y, threshold, reg) is True


# deferred: draw — command/UI layer
# deferred: get_type_name / can_apply_to — command/UI layer

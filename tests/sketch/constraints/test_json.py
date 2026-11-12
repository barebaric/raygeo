"""Round-trip and solver-math tests for constraint dictionaries
parsed through EntityRegistry.add_constraint_json."""

from raygeo.sketch.registry import EntityRegistry


def _registry_with_square():
    reg = EntityRegistry()
    for x, y in [(0, 0), (60, 0), (60, 30), (0, 30)]:
        reg.add_point(float(x), float(y), False)
    reg.add_line(0, 1, False)
    reg.add_line(1, 2, False)
    reg.add_line(2, 3, False)
    reg.add_line(3, 0, False)
    reg.add_circle(0, 1, False)
    return reg


def _point_constraint_dicts():
    return [
        {
            "type": "HorizontalConstraint",
            "p1": 0,
            "p2": 1,
            "user_visible": True,
        },
        {"type": "VerticalConstraint", "p1": 1, "p2": 2, "user_visible": True},
        {
            "type": "CoincidentConstraint",
            "p1": 2,
            "p2": 3,
            "user_visible": True,
        },
        {
            "type": "CollinearConstraint",
            "p1": 0,
            "p2": 1,
            "p3": 2,
            "user_visible": True,
        },
        {
            "type": "EqualDistanceConstraint",
            "p1": 0,
            "p2": 1,
            "p3": 2,
            "p4": 3,
            "user_visible": True,
        },
        {
            "type": "DistanceConstraint",
            "p1": 0,
            "p2": 1,
            "value": 60.0,
            "user_visible": True,
        },
    ]


def test_add_and_round_trip_point_constraints():
    reg = _registry_with_square()
    for index, constraint in enumerate(_point_constraint_dicts()):
        handle = reg.add_constraint_json(constraint)
        assert handle == index
        assert reg.constraint_dict(index) == constraint
    assert reg.constraint_count() == len(_point_constraint_dicts())


def test_residuals_match_expected_math():
    reg = _registry_with_square()
    for constraint in _point_constraint_dicts():
        reg.add_constraint_json(constraint)
    residuals = reg.constraint_residuals()
    # Horizontal(0,1): y0-y1=0. Vertical(1,2): x1-x2=0.
    # Coincident(2,3): x: 60-0=60, y: 30-30=0.
    # Collinear(0,1,2): cross of (60,0)x(60,30) = 1800.
    # EqualDistance(0,1,2,3): 60-60=0. Distance(0,1,60): 60-60=0.
    assert residuals == [0.0, 0.0, 60.0, 0.0, 1800.0, 0.0, 0.0]


def test_radius_constraint_resolves_entity_points():
    reg = EntityRegistry()
    center = reg.add_point(5.0, 5.0, False)
    radius_pt = reg.add_point(15.0, 5.0, False)
    circle = reg.add_circle(center, radius_pt, False)
    handle = reg.add_constraint_json(
        {
            "type": "RadiusConstraint",
            "entity_id": circle,
            "value": 10.0,
            "user_visible": True,
        }
    )
    assert reg.constraint_residuals() == [0.0]
    reg.set_point_position(radius_pt, 25.0, 5.0)
    assert reg.constraint_residuals() == [10.0]
    assert reg.constraint_dict(handle) == {
        "type": "RadiusConstraint",
        "entity_id": circle,
        "value": 10.0,
        "user_visible": True,
    }


def test_diameter_constraint_resolves_circle_points():
    reg = EntityRegistry()
    center = reg.add_point(0.0, 0.0, False)
    radius_pt = reg.add_point(4.0, 0.0, False)
    circle = reg.add_circle(center, radius_pt, False)
    reg.add_constraint_json(
        {
            "type": "DiameterConstraint",
            "circle_id": circle,
            "value": 8.0,
            "user_visible": True,
        }
    )
    assert reg.constraint_residuals() == [0.0]


def test_perpendicular_line_line_resolves():
    reg = EntityRegistry()
    a1 = reg.add_point(0.0, 0.0, False)
    a2 = reg.add_point(10.0, 0.0, False)
    b1 = reg.add_point(1.0, 1.0, False)
    b2 = reg.add_point(1.0, 3.0, False)
    reg.add_line(a1, a2, False)
    reg.add_line(b1, b2, False)
    reg.add_constraint_json(
        {
            "type": "PerpendicularConstraint",
            "e1_id": 4,
            "e2_id": 5,
            "user_visible": True,
        }
    )
    assert reg.constraint_residuals() == [0.0]


def test_tangent_constraint_resolves():
    reg = EntityRegistry()
    l1 = reg.add_point(0.0, 0.0, False)
    l2 = reg.add_point(10.0, 0.0, False)
    line = reg.add_line(l1, l2, False)
    center = reg.add_point(5.0, 2.0, False)
    radius_pt = reg.add_point(5.0, 5.0, False)
    circle = reg.add_circle(center, radius_pt, False)
    reg.add_constraint_json(
        {
            "type": "TangentConstraint",
            "line_id": line,
            "shape_id": circle,
            "user_visible": True,
        }
    )
    assert reg.constraint_residuals() == [-1.0]


def test_point_on_line_resolves():
    reg = EntityRegistry()
    point = reg.add_point(1.0, 2.0, False)
    l1 = reg.add_point(0.0, 0.0, False)
    l2 = reg.add_point(2.0, 0.0, False)
    line = reg.add_line(l1, l2, False)
    reg.add_constraint_json(
        {
            "type": "PointOnLineConstraint",
            "point_id": point,
            "shape_id": line,
            "user_visible": True,
        }
    )
    assert reg.constraint_residuals() == [2.0]


def test_angle_constraint_resolves():
    reg = EntityRegistry()
    p1 = reg.add_point(0.0, 0.0, False)
    p2 = reg.add_point(4.0, 0.0, False)
    p3 = reg.add_point(4.0, 0.0, False)
    p4 = reg.add_point(4.0, 4.0, False)
    e1 = reg.add_line(p1, p2, False)
    e2 = reg.add_line(p3, p4, False)
    reg.add_constraint_json(
        {
            "type": "AngleConstraint",
            "e1_id": e1,
            "e2_id": e2,
            "value": 90.0,
            "user_visible": True,
            "e1_far_idx": None,
            "e2_far_idx": None,
        }
    )
    assert abs(reg.constraint_residuals()[0]) < 1e-9


def test_remove_constraint():
    reg = EntityRegistry()
    p1 = reg.add_point(0.0, 2.0, False)
    p2 = reg.add_point(5.0, 6.0, False)
    reg.add_constraint_json(
        {
            "type": "HorizontalConstraint",
            "p1": p1,
            "p2": p2,
            "user_visible": True,
        }
    )
    assert reg.constraint_count() == 1
    reg.remove_constraint(0)
    assert reg.constraint_count() == 0

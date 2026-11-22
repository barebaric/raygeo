import pytest

from raygeo.sketch import Sketch
from raygeo.sketch.constraints import DistanceConstraint


def _line_document() -> Sketch:
    doc = Sketch("test")
    p1 = doc.add_point(0.0, 0.0, fixed=True)
    p2 = doc.add_point(10.0, 2.0)
    doc.add_line(p1, p2)
    doc.constrain_distance(p1, p2, 10.0)
    doc.constrain_horizontal(p1, p2)
    return doc


def test_document_starts_with_origin():
    doc = Sketch("test")

    assert doc.name == "test"
    assert len(doc.registry.points()) == 1
    assert doc.origin_id in [p.id for p in doc.registry.points()]


def test_document_solve_and_geometry():
    doc = _line_document()

    assert doc.solve() is True
    p2 = doc.registry.get_point(2)
    assert p2.x == pytest.approx(10.0)
    assert p2.y == pytest.approx(0.0, abs=1e-6)
    assert not doc.to_geometry().is_empty()


def test_document_serialization_round_trip():
    doc = _line_document()
    doc.solve()

    data = doc.to_dict()
    restored = Sketch.from_dict(data)

    assert restored.uid == doc.uid
    assert restored.name == doc.name
    assert restored.registry.to_dict() == doc.registry.to_dict()
    assert len(restored.constraints) == len(doc.constraints)


def test_clone_for_geometry_is_independent():
    doc = _line_document()
    clone = doc.clone_for_geometry()
    clone.registry.get_point(2).x = 99.0

    assert doc.registry.get_point(2).x == 10.0
    assert clone.registry.get_point(2).x == 99.0


def test_undo_state_restores_points_and_entities():
    doc = _line_document()
    state = doc.capture_undo_state()
    p2 = doc.registry.get_point(2)
    p2.x = 42.0
    p2.y = 42.0

    doc.apply_undo_state(state)

    restored = doc.registry.get_point(2)
    assert restored.x == pytest.approx(10.0)
    assert restored.y == pytest.approx(2.0)


def test_constraints_are_a_rust_owned_list():
    doc = Sketch("test")
    p1 = doc.add_point(0.0, 0.0)
    p2 = doc.add_point(10.0, 0.0)
    constraint = DistanceConstraint(p1, p2, 10.0)

    doc.constraints.append(constraint)

    assert doc.constraints[0] is constraint
    assert doc.constraints[-1] is constraint
    assert doc.constraints[:] == [constraint]
    assert constraint in doc.constraints
    assert doc.constraints.index(constraint) == 0
    assert doc.constraints.count(constraint) == 1
    assert list(doc.constraints) == [constraint]
    assert doc.constraints == [constraint]
    assert len(doc.constraints) == 1

    doc.constraints.remove(constraint)
    assert doc.constraints == []

    doc.constraints = [constraint]
    assert doc.constraints == [constraint]

    doc.constraints.clear()
    assert not doc.constraints


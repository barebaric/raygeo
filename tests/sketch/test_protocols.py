"""Guards the nominal model types and their Python surfaces.

Every class registered under ``raygeo.sketch.constraints`` must be a
subclass of ``Constraint``, every entity class under
``raygeo.sketch.entities`` a subclass of ``Entity``, and each must
expose the shared binding surface.  Enumeration is dynamic: a newly
added model class is picked up without touching this test, and a
missing base or member names the gap.
"""

import sys

from raygeo.sketch.constraints import Constraint
from raygeo.sketch.entities import Entity

CONSTRAINT_MEMBERS = {
    "status",
    "referenced_points",
    "get_referenced_point_ids",
    "to_dict",
    "error",
    "draw",
    "is_hit",
}

ENTITY_MEMBERS = {
    "id",
    "type_name",
    "get_point_ids",
    "construction",
    "invisible",
    "array_copy",
    "constrained",
}


def _registered_classes(prefix: str) -> list[tuple[str, type]]:
    classes = []
    seen: set[int] = set()
    for name, module in list(sys.modules.items()):
        if not name.startswith(prefix + "."):
            continue
        for attr, obj in list(vars(module).items()):
            if not isinstance(obj, type) or id(obj) in seen:
                continue
            if not attr.endswith("Constraint"):
                continue  # target enums and helpers are not constraints
            if attr == "Constraint":
                continue  # the base itself
            seen.add(id(obj))
            classes.append((f"{name}.{attr}", obj))
    return classes


def _entity_classes() -> list[tuple[str, type]]:
    classes = []
    seen: set[int] = set()
    for name, module in list(sys.modules.items()):
        if not name.startswith("raygeo.sketch.entities."):
            continue
        for attr, obj in list(vars(module).items()):
            if not isinstance(obj, type) or id(obj) in seen:
                continue
            if attr in ("Entity", "SketchPoint"):
                continue  # the base itself and the point model
            seen.add(id(obj))
            classes.append((f"{name}.{attr}", obj))
    return classes


def test_every_constraint_class_is_a_constraint():
    classes = _registered_classes("raygeo.sketch.constraints")
    assert classes, "no constraint classes found — enumeration broke"
    for qualified, cls in classes:
        assert issubclass(cls, Constraint), f"{qualified} is not a Constraint"
        missing = [m for m in CONSTRAINT_MEMBERS if not hasattr(cls, m)]
        assert not missing, f"{qualified} misses {missing}"


def test_every_entity_class_is_an_entity():
    classes = _entity_classes()
    assert classes, "no entity classes found — enumeration broke"
    for qualified, cls in classes:
        assert issubclass(cls, Entity), f"{qualified} is not an Entity"
        missing = [m for m in ENTITY_MEMBERS if not hasattr(cls, m)]
        assert not missing, f"{qualified} misses {missing}"


def test_entity_id_alias_is_wired():
    from raygeo.sketch.types import EntityID

    assert EntityID is int

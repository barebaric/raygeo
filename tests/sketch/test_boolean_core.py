from raygeo.sketch.boolean import (
    BooleanOp,
    BooleanRegion,
    apply_boolean,
    build_boolean_regions,
    build_boolean_regions_from_constraints,
)
from raygeo.sketch.registry import EntityRegistry


def _two_squares():
    registry = EntityRegistry()
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    square = registry.add_polygon(
        a,
        b,
        [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        closed=True,
    )
    e = registry.add_point(10.0, 0.0, False)
    f = registry.add_point(20.0, 0.0, False)
    square2 = registry.add_polygon(
        e,
        f,
        [(10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (10.0, 10.0)],
        closed=True,
    )
    return registry, [square, square2]


def test_build_and_union_regions():
    registry, ids = _two_squares()
    regions = build_boolean_regions(registry, ids)
    assert regions is not None
    assert len(regions) == 2
    solids = apply_boolean(BooleanOp.UNION, regions)
    assert solids is not None
    assert len(solids) >= 1


def test_apply_boolean_accepts_descriptors():
    region = BooleanRegion([[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]], [1])
    solids = apply_boolean(BooleanOp.UNION, [region])
    assert solids is not None
    assert len(solids) >= 1


def test_build_regions_from_constraints():
    registry, ids = _two_squares()
    regions = build_boolean_regions_from_constraints(registry, ids, [])
    assert regions is not None
    assert len(regions) == 2
    solids = apply_boolean(BooleanOp.UNION, regions)
    assert solids is not None


def test_union_of_touching_squares_yields_one_solid():
    """Two squares sharing the full edge x=10 union into one solid
    (port of the boolean core's raw-ring union case)."""
    a = BooleanRegion(
        [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]], [1]
    )
    b = BooleanRegion(
        [[(10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (10.0, 10.0)]], [2]
    )
    solids = apply_boolean(BooleanOp.UNION, [a, b])
    assert solids is not None
    assert len(solids) == 1

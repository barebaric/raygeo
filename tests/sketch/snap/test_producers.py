"""Tests for the snap producers through their Python bindings."""

import pytest

from raygeo.sketch.registry import EntityRegistry
from raygeo.sketch.snap.producers import (
    CentersProducer,
    EntityPointsProducer,
    EquidistantLinesProducer,
    IntersectionsProducer,
    MidpointsProducer,
    OnEntityProducer,
)
from raygeo.sketch.snap.types import DragContext


@pytest.fixture
def registry():
    return EntityRegistry()


def _two_crossing_lines(registry):
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    c = registry.add_point(5.0, -5.0, False)
    d = registry.add_point(5.0, 5.0, False)
    return (
        registry.add_line(a, b, False),
        registry.add_line(c, d, False),
    )


def test_entity_points_lines_cover_both_axes(registry):
    p1 = registry.add_point(3.0, 4.0, False)
    registry.add_point(1.0, 2.0, False)
    lines = EntityPointsProducer().produce(
        registry, (0.0, 0.0), DragContext(), 5.0
    )
    coords = {line.coordinate for line in lines}
    assert {3.0, 4.0, 1.0, 2.0} <= coords
    horizontal = [line for line in lines if line.is_horizontal]
    assert all(line.coordinate in (4.0, 2.0) for line in horizontal)


def test_entity_points_exclude_dragged_points(registry):
    p1 = registry.add_point(3.0, 4.0, False)
    registry.add_point(1.0, 2.0, False)
    context = DragContext(dragged_point_ids={p1})
    lines = EntityPointsProducer().produce(registry, (0.0, 0.0), context, 5.0)
    coords = {line.coordinate for line in lines}
    assert 3.0 not in coords
    assert 4.0 not in coords
    assert 1.0 in coords


def test_entity_points_suppress_coincident_candidates(registry):
    registry.add_point(3.0, 4.0, False)
    dragged = registry.add_point(3.0000001, 4.0, False)
    context = DragContext(dragged_point_ids={dragged})
    lines = EntityPointsProducer().produce(registry, (0.0, 0.0), context, 5.0)
    coords = {line.coordinate for line in lines}
    assert 3.0 not in coords and 4.0 not in coords


def test_centers_from_circle_and_arc(registry):
    center = registry.add_point(0.0, 0.0, False)
    rx = registry.add_point(5.0, 0.0, False)
    circle = registry.add_circle(center, rx, False)
    start = registry.add_point(10.0, 0.0, False)
    end = registry.add_point(0.0, 10.0, False)
    arc_center = registry.add_point(0.0, 0.0, False)
    registry.add_arc(start, end, arc_center, clockwise=False)

    lines = CentersProducer().produce(registry, (0.0, 0.0), DragContext(), 5.0)
    vertical = {line.coordinate for line in lines if not line.is_horizontal}
    assert vertical == {0.0}

    points = CentersProducer().produce_points(
        registry, (0.0, 0.0), DragContext(), 5.0
    )
    assert len(points) == 2
    assert all(point.x == 0.0 and point.y == 0.0 for point in points)


def test_midpoints_line_and_arc(registry):
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    registry.add_line(a, b, False)
    center = registry.add_point(0.0, 0.0, False)
    start = registry.add_point(10.0, 0.0, False)
    end = registry.add_point(0.0, 10.0, False)
    registry.add_arc(start, end, center, clockwise=False)

    points = MidpointsProducer().produce_points(
        registry, (0.0, 0.0), DragContext(), 100.0
    )
    coords = sorted((point.x, point.y) for point in points)
    assert (5.0, 0.0) in coords


def test_on_entity_nearest_point(registry):
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    registry.add_line(a, b, False)

    points = OnEntityProducer().produce_points(
        registry, (4.5, 2.0), DragContext(), 3.0
    )
    assert len(points) == 1
    assert points[0].x == pytest.approx(4.5)
    assert points[0].y == 0.0


def test_on_entity_respects_threshold(registry):
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    registry.add_line(a, b, False)

    points = OnEntityProducer().produce_points(
        registry, (4.5, 5.0), DragContext(), 1.0
    )
    assert points == []


def test_intersections_line_pair(registry):
    _l1, _l2 = _two_crossing_lines(registry)

    points = IntersectionsProducer().produce_points(
        registry, (5.0, 0.0), DragContext(), 1.0
    )
    assert len(points) == 1
    assert points[0].x == pytest.approx(5.0)
    assert points[0].y == pytest.approx(0.0)


def test_intersections_cache_hit(registry):
    _l1, _l2 = _two_crossing_lines(registry)
    producer = IntersectionsProducer()
    first = producer.produce(registry, (0.0, 0.0), DragContext(), 5.0)
    assert producer._cache_key is not None
    second = producer.produce(registry, (0.0, 0.0), DragContext(), 5.0)
    assert [line.coordinate for line in first] == [
        line.coordinate for line in second
    ]


def test_equidistant_between_aligned_points(registry):
    registry.add_point(0.0, 0.0, False)
    registry.add_point(0.0, 4.0, False)
    registry.add_point(0.0, 8.0, False)

    points = EquidistantLinesProducer().produce_points(
        registry, (0.0, 4.2), DragContext(), 1.5
    )
    assert points, "expected an equidistant snap near the middle row"
    snapped = points[0]
    assert snapped.is_horizontal
    assert snapped.y == pytest.approx(4.0)
    assert snapped.spacing == pytest.approx(4.0)
    assert snapped.axis_coord == pytest.approx(0.0)


def test_equidistant_ignores_far_off_axis(registry):
    registry.add_point(0.0, 0.0, False)
    registry.add_point(0.0, 4.0, False)

    points = EquidistantLinesProducer().produce_points(
        registry, (50.0, 2.0), DragContext(), 1.5
    )
    assert points == []


def test_suppression_off_keeps_coinciding_candidate(registry):
    """Creation-time snapping disables coincident-point suppression:
    a candidate that merely *coincides spatially* with an excluded
    preview point (different id) must still be offered.

    Regression: the suppression check was ported inverted, so with
    suppression off every candidate was filtered and the path tool
    could no longer close onto its start point."""
    target = registry.add_point(3.0, 4.0, False)
    registry.add_point(30.0, 30.0, False)
    # A preview temp point sitting on the target's position, under a
    # different id — the path tool's shape at the closing click.
    temp = registry.add_point(3.0, 4.0, False)
    context = DragContext(
        dragged_point_ids={temp}, suppress_coincident_points=False
    )

    points = EntityPointsProducer().produce_points(
        registry, (3.0, 4.0), context, 100.0
    )
    coords = {(point.x, point.y) for point in points}
    assert (3.0, 4.0) in coords
    assert (30.0, 30.0) in coords

    lines = EntityPointsProducer().produce(registry, (3.0, 4.0), context, 5.0)
    coords = {line.coordinate for line in lines}
    assert {3.0, 4.0, 30.0} <= coords
    assert target is not None


def test_suppression_on_filters_coinciding_candidate(registry):
    """With suppression enabled (the drag default), a candidate that
    coincides with an excluded point is filtered; the untouched
    candidate is still offered."""
    registry.add_point(3.0, 4.0, False)
    registry.add_point(30.0, 30.0, False)
    temp = registry.add_point(3.0, 4.0, False)
    context = DragContext(
        dragged_point_ids={temp}, suppress_coincident_points=True
    )

    points = EntityPointsProducer().produce_points(
        registry, (3.0, 4.0), context, 100.0
    )
    coords = {(point.x, point.y) for point in points}
    assert (3.0, 4.0) not in coords
    assert (30.0, 30.0) in coords

    lines = EntityPointsProducer().produce(registry, (3.0, 4.0), context, 5.0)
    coords = {line.coordinate for line in lines}
    assert 3.0 not in coords and 4.0 not in coords
    assert 30.0 in coords

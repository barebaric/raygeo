"""Tests for the snap value types."""

import pytest

from raygeo.sketch.registry import EntityRegistry
from raygeo.sketch.snap.types import (
    SNAP_LINE_STYLES,
    DragContext,
    SnapLine,
    SnapLineStyle,
    SnapLineType,
    SnapPoint,
    SnapResult,
)


@pytest.fixture
def registry():
    return EntityRegistry()


def test_snap_line_type_ordinals_and_names():
    assert SnapLineType.ENTITY_POINT.value == 0
    assert SnapLineType.CENTER.value == 6
    assert SnapLineType.ENTITY_POINT.name == "ENTITY_POINT"
    assert SnapLineType.MIDPOINT.name == "MIDPOINT"
    assert SnapLineType.ENTITY_POINT.priority > SnapLineType.CENTER.priority


def test_priority_order_matches_reference():
    order = [
        SnapLineType.ENTITY_POINT,
        SnapLineType.MIDPOINT,
        SnapLineType.ON_ENTITY,
        SnapLineType.INTERSECTION,
        SnapLineType.EQUIDISTANT,
        SnapLineType.TANGENT,
        SnapLineType.CENTER,
    ]
    priorities = [kind.priority for kind in order]
    assert priorities == sorted(priorities, reverse=True)


def test_style_table_covers_all_kinds():
    assert len(SNAP_LINE_STYLES) == 7
    for kind, style in SNAP_LINE_STYLES.items():
        assert isinstance(style, SnapLineStyle)
        assert len(style.color) == 4
        assert style.line_width > 0.0


def test_snap_point_value_surface():
    point = SnapPoint(1.0, 2.0, SnapLineType.MIDPOINT, spacing=0.5)
    assert point.pos == (1.0, 2.0)
    assert point.spacing == 0.5
    assert point.line_type.name == "MIDPOINT"
    assert "MIDPOINT" in repr(point)


def test_snap_line_value_surface():
    line = SnapLine(
        is_horizontal=True, coordinate=3.0, line_type=SnapLineType.CENTER
    )
    assert line.is_horizontal is True
    assert line.coordinate == 3.0


def test_snap_result_constructors():
    no_snap = SnapResult.no_snap((4.0, 5.0))
    assert no_snap.snapped is False
    assert no_snap.position == (4.0, 5.0)
    assert no_snap.distance == float("inf")

    point = SnapPoint(1.0, 1.0, SnapLineType.ENTITY_POINT)
    from_point = SnapResult.from_snap_point(point, 0.5)
    assert from_point.snapped is True
    assert from_point.primary_snap_point is not None
    assert from_point.distance == pytest.approx(0.5)

    line = SnapLine(
        is_horizontal=False, coordinate=1.0, line_type=SnapLineType.CENTER
    )
    from_line = SnapResult.from_snap_line(line, (2.0, 2.0), 1.0)
    assert from_line.snapped is True
    # Snapping to x=1 pulls the position onto the guide.
    assert from_line.position[0] == pytest.approx(1.0)


def test_drag_context_defaults_and_membership():
    context = DragContext()
    assert context.suppress_coincident_points is True
    assert context.dragged_point_ids == set()

    context = DragContext(
        dragged_point_ids={1, 2},
        dragged_entity_ids={7},
        preview_entity_ids={9},
    )
    assert context.dragged_point_ids == {1, 2}
    assert context.dragged_entity_ids == {7}
    assert context.preview_entity_ids == {9}


def test_drag_context_coincides_with_dragged(registry):
    pid = registry.add_point(3.0, 4.0, False)
    context = DragContext(dragged_point_ids={pid})
    assert context.coincides_with_dragged(3.0, 4.0, registry, 1e-6)
    assert not context.coincides_with_dragged(5.0, 5.0, registry, 1e-6)

    unsuppressed = DragContext(
        dragged_point_ids={pid}, suppress_coincident_points=False
    )
    assert not unsuppressed.coincides_with_dragged(3.0, 4.0, registry, 1e-6)

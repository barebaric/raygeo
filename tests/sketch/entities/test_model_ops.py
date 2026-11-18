"""Offset, placement and waypoint operations on the compiled model."""

import math

import pytest

from raygeo.sketch.entities import (
    Bezier,
    OffsetPlan,
    PolygonOutline,
    SketchPoint,
)
from raygeo.sketch.registry import EntityRegistry


class _Sketch:
    def __init__(self, registry: EntityRegistry) -> None:
        self.registry = registry


class _Placement:
    def __init__(self, mirror: bool = False) -> None:
        self.mirror = mirror

    def transform_offset(self, dx: float, dy: float) -> tuple[float, float]:
        return (-dx, dy) if self.mirror else (dx, dy)


def _plan_moves(plan: OffsetPlan) -> dict:
    return plan.point_moves


class TestEntityOffsets:
    def test_circle_offsets_in_place(self):
        registry = EntityRegistry()
        center = registry.add_point(10.0, 10.0)
        radius_pt = registry.add_point(15.0, 10.0)
        circle = registry.get_entity(registry.add_circle(center, radius_pt))

        plan = circle.plan_offset(registry, 2.0, lambda: -1)

        assert plan is not None
        assert _plan_moves(plan)[radius_pt] == pytest.approx((17.0, 10.0))
        assert circle.preview_polylines(registry, 2.0)
        assert circle.as_offset_item(_Sketch(registry)) is not None

    def test_arc_offsets_both_endpoints(self):
        registry = EntityRegistry()
        center = registry.add_point(0.0, 0.0)
        start = registry.add_point(10.0, 0.0)
        end = registry.add_point(0.0, 10.0)
        arc = registry.get_entity(
            registry.add_arc(start, end, center, clockwise=False)
        )

        plan = arc.plan_offset(registry, 1.0, lambda: -1)

        assert plan is not None
        moves = _plan_moves(plan)
        assert moves[start] == pytest.approx((11.0, 0.0))
        assert moves[end] == pytest.approx((0.0, 11.0))
        assert arc.preview_polylines(registry, 1.0)

    def test_ellipse_offsets_both_radii(self):
        registry = EntityRegistry()
        center = registry.add_point(0.0, 0.0)
        rx_pt = registry.add_point(10.0, 0.0)
        ry_pt = registry.add_point(0.0, 5.0)
        ellipse = registry.get_entity(
            registry.add_ellipse(center, rx_pt, ry_pt)
        )

        plan = ellipse.plan_offset(registry, 1.0, lambda: -1)

        assert plan is not None
        moves = _plan_moves(plan)
        assert moves[rx_pt] == pytest.approx((11.0, 0.0))
        assert moves[ry_pt] == pytest.approx((0.0, 6.0))
        assert ellipse.preview_polylines(registry, 1.0)

    def test_closed_polygon_offsets_as_entity(self):
        registry = EntityRegistry()
        center = registry.add_point(5.0, 5.0)
        handle = registry.add_point(10.0, 5.0)
        polygon = registry.get_entity(
            registry.add_polygon(
                center,
                handle,
                [(-5.0, -5.0), (5.0, -5.0), (5.0, 5.0), (-5.0, 5.0)],
                closed=True,
            )
        )

        plan = polygon.plan_offset(registry, 1.0, lambda: -1)

        assert plan is not None
        assert list(plan.removed_entity_ids) == [polygon.id]
        assert len(plan.entities) == 1
        assert len(plan.points) == 2
        assert polygon.preview_polylines(registry, 1.0)
        assert polygon.as_offset_item(_Sketch(registry)) is not None

    def test_open_polygon_offsets_as_outline(self):
        registry = EntityRegistry()
        center = registry.add_point(0.0, 0.0)
        handle = registry.add_point(1.0, 0.0)
        polygon = registry.get_entity(
            registry.add_polygon(
                center, handle, [(0.0, 0.0), (10.0, 0.0)], closed=False
            )
        )

        item = polygon.as_offset_item(_Sketch(registry))

        assert isinstance(item, PolygonOutline)
        assert item.closed is False
        plan = item.plan_offset(registry, 1.0, lambda: -1)
        assert plan is not None
        assert len(plan.entities) == 1
        assert item.preview_polylines(registry, 1.0)

    def test_polygon_outline_helpers(self):
        outline = PolygonOutline(
            [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            True,
            [7],
        )

        assert outline.offset(1.0, 0.02)
        assert outline.preview_polylines(None, 1.0)

    def test_polygon_frame_helpers(self):
        registry = EntityRegistry()
        center = registry.add_point(0.0, 0.0)
        handle = registry.add_point(1.0, 0.0)
        polygon = registry.get_entity(
            registry.add_polygon(
                center, handle, [(0.0, 0.0), (2.0, 0.0)], closed=False
            )
        )

        local = polygon.normalize_vertices(
            (0.0, 0.0), (1.0, 0.0), [(0.0, 0.0), (2.0, 0.0)]
        )
        assert local == [
            pytest.approx((0.0, 0.0)),
            pytest.approx((2.0, 0.0)),
        ]
        vertices, closed = polygon.to_outline(registry)
        assert closed is False
        assert vertices == [
            pytest.approx((0.0, 0.0)),
            pytest.approx((2.0, 0.0)),
        ]


class TestPlacementOffsets:
    def test_bezier_transform_offsets(self):
        registry = EntityRegistry()
        start = registry.add_point(0.0, 0.0)
        end = registry.add_point(10.0, 0.0)
        bezier = registry.get_entity(
            registry.add_bezier(start, end, cp1=(2.0, 3.0))
        )

        bezier.transform_offsets(_Placement())

        assert bezier.cp1 == pytest.approx((2.0, 3.0))

    def test_bezier_rewrite_offsets_from(self):
        registry = EntityRegistry()
        start = registry.add_point(0.0, 0.0)
        end = registry.add_point(10.0, 0.0)
        template = registry.get_entity(
            registry.add_bezier(start, end, cp1=(2.0, 3.0))
        )
        copy = Bezier(99, start, end, cp1=(1.0, 1.0))

        copy.rewrite_offsets_from(template, _Placement(mirror=True))

        assert copy.cp1 == pytest.approx((-2.0, 3.0))

    def test_polygon_transform_offsets_mirrors_rings(self):
        registry = EntityRegistry()
        center = registry.add_point(0.0, 0.0)
        handle = registry.add_point(1.0, 0.0)
        polygon = registry.get_entity(
            registry.add_polygon(
                center,
                handle,
                [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0)],
                closed=True,
            )
        )

        polygon.transform_offsets(_Placement(mirror=True))

        assert sorted(polygon.vertices) == [
            pytest.approx((0.0, 0.0)),
            pytest.approx((2.0, -1.0)),
            pytest.approx((2.0, 0.0)),
        ]


class TestTextBoxFrame:
    def test_frame_polygon_and_natural_size(self):
        registry = EntityRegistry()
        origin = registry.add_point(0.0, 0.0)
        width = registry.add_point(10.0, 0.0)
        height = registry.add_point(0.0, 5.0)
        text_box = registry.get_entity(
            registry.add_text_box(origin, width, height, "Hi")
        )

        assert text_box.get_frame_polygon(registry) == [
            pytest.approx((0.0, 0.0)),
            pytest.approx((10.0, 0.0)),
            pytest.approx((10.0, 5.0)),
            pytest.approx((0.0, 5.0)),
        ]
        natural_w, natural_h = text_box.get_natural_size(None)
        assert natural_w > 0.0
        assert natural_h > 0.0
        assert len(text_box.get_font_metrics()) == 3


class TestPointWaypoints:
    def test_symmetric_constraint_mirrors_control_point(self):
        registry = EntityRegistry()
        p1 = registry.add_point(0.0, 0.0)
        p2 = registry.add_point(10.0, 10.0)
        p3 = registry.add_point(20.0, 0.0)
        b1 = registry.get_entity(registry.add_bezier(p1, p2))
        b2 = registry.get_entity(registry.add_bezier(p2, p3))
        point = registry.get_point(p2)
        point.waypoint_type = "symmetric"
        b1.cp2 = (5.0, 3.0)

        point.apply_constraint(registry, b1, 2)

        b2 = registry.get_entity(b2.id)
        assert b2.cp1 == pytest.approx((-5.0, -3.0))

    def test_smooth_constraint_preserves_other_length(self):
        registry = EntityRegistry()
        p1 = registry.add_point(0.0, 0.0)
        p2 = registry.add_point(10.0, 10.0)
        p3 = registry.add_point(20.0, 0.0)
        b1 = registry.get_entity(registry.add_bezier(p1, p2))
        b2 = registry.get_entity(registry.add_bezier(p2, p3))
        point = registry.get_point(p2)
        point.waypoint_type = "smooth"
        b2.cp1 = (2.0, 4.0)
        b1.cp2 = (6.0, 8.0)

        point.apply_constraint(registry, b1, 2)

        b2 = registry.get_entity(b2.id)
        assert b2.cp1 is not None
        assert math.hypot(*b2.cp1) == pytest.approx(
            math.hypot(2.0, 4.0)
        )
        assert b2.cp1[0] < 0
        assert b2.cp1[1] < 0

    def test_sharp_point_is_a_noop(self):
        registry = EntityRegistry()
        p1 = registry.add_point(0.0, 0.0)
        p2 = registry.add_point(10.0, 10.0)
        p3 = registry.add_point(20.0, 0.0)
        b1 = registry.get_entity(registry.add_bezier(p1, p2))
        b2 = registry.get_entity(registry.add_bezier(p2, p3))
        b1.cp2 = (5.0, 3.0)

        registry.get_point(p2).apply_constraint(registry, b1, 2)

        assert b2.cp1 is None

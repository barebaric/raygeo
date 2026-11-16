from __future__ import annotations

import math
from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

import cairo
from gi.repository import PangoCairo

from raygeo.geo.shape.circle import get_circle_circle_intersections
from raygeo.geo.shape.line import (
    get_line_line_intersection,
    is_point_on_line_segment,
)
from raygeo.geo.types import Point
from raygeo.sketch.constraints.perpendicular import (
    PerpendicularConstraint as _CorePerpendicularConstraint,
)
from raygeo.sketch.entities import Arc, Circle, Line

from raygeo.sketch.model.text import create_text_layout

from .base import Constraint, ConstraintStatus

if TYPE_CHECKING:
    from raygeo.sketch.entities import Entity
    from raygeo.sketch.model.protocols import (
        ParameterContext,
        Sketch,
        SketchSelection,
    )
    from raygeo.sketch.registry import EntityRegistry
    from raygeo.sketch.types import EntityID


class PerpendicularConstraint(_CorePerpendicularConstraint, Constraint):
    """
    Enforces perpendicularity between two entities.
    - Line/Line: Vectors are at 90 degrees.
    - Line/Arc, Line/Circle: Line passes through the shape's center.
    - Arc/Arc, Arc/Circle, Circle/Circle: Shapes intersect at a right angle.
    """

    def __new__(cls, e1_id, e2_id, user_visible=True):
        return super().__new__(cls, e1_id, e2_id, user_visible)

    def __init__(
        self, e1_id: EntityID, e2_id: EntityID, user_visible: bool = True
    ):
        self.e1_id: EntityID = e1_id
        self.e2_id: EntityID = e2_id

    @classmethod
    def get_type_key(cls) -> str:
        return "perp"

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        if selection.point_ids or len(selection.entity_ids) != 2:
            return False
        entities = selection.resolve_entities(
            sketch.registry if sketch else None
        )
        return entities is not None and cls.applies_to_entities(entities)

    @classmethod
    def applies_to_entities(cls, entities: Sequence[Entity]) -> bool:
        """Both operands must be Lines or radius-bearing entities."""
        return len(entities) == 2 and all(
            isinstance(e, Line) or e.is_radius_entity() for e in entities
        )

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Perpendicular")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: EntityRegistry) -> str:
        """Returns subtitle describing constrained entities."""
        e1 = registry.get_entity(self.e1_id)
        e2 = registry.get_entity(self.e2_id)
        if e1 and e2:
            return _("Between {} and {}").format(
                type(e1).__name__, type(e2).__name__
            )
        return ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> PerpendicularConstraint:
        return cls(
            e1_id=data["e1_id"],
            e2_id=data["e2_id"],
            user_visible=data.get("user_visible", True),
        )

    def get_visuals(
        self,
        reg: EntityRegistry,
        to_screen: Callable[[Point], Point],
    ) -> tuple[float, float, float | None, float | None] | None:
        """Calculates screen position and angles for visualization."""
        e1 = reg.get_entity(self.e1_id)
        e2 = reg.get_entity(self.e2_id)
        if not (e1 and e2):
            return None

        # --- Case 1: Line-Line ---
        if isinstance(e1, Line) and isinstance(e2, Line):
            return self._get_line_line_visuals(e1, e2, reg, to_screen)

        # --- Case 2: Line-Shape ---
        line, shape = (e1, e2) if isinstance(e1, Line) else (e2, e1)
        if isinstance(line, Line) and isinstance(shape, (Arc, Circle)):
            return self._get_line_shape_visuals(line, shape, reg, to_screen)

        # --- Case 3: Shape-Shape ---
        if isinstance(e1, (Arc, Circle)) and isinstance(e2, (Arc, Circle)):
            return self._get_shape_shape_visuals(e1, e2, reg, to_screen)

        return None

    def _get_line_line_visuals(self, l1, l2, reg, to_screen):
        p1 = reg.get_point(l1.p1_idx)
        p2 = reg.get_point(l1.p2_idx)
        p3 = reg.get_point(l2.p1_idx)
        p4 = reg.get_point(l2.p2_idx)
        pt = get_line_line_intersection(
            (p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y), (p4.x, p4.y)
        )
        if not pt:
            m1x, m1y = (p1.x + p2.x) / 2, (p1.y + p2.y) / 2
            m2x, m2y = (p3.x + p4.x) / 2, (p3.y + p4.y) / 2
            pt = ((m1x + m2x) / 2, (m1y + m2y) / 2)

        ix, iy = pt
        sx, sy = to_screen((ix, iy))
        s_p1, s_p2 = to_screen((p1.x, p1.y)), to_screen((p2.x, p2.y))
        s_p3, s_p4 = to_screen((p3.x, p3.y)), to_screen((p4.x, p4.y))

        def dist_sq(a, b):
            return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2

        v1p = (
            s_p1 if dist_sq(s_p1, (sx, sy)) > dist_sq(s_p2, (sx, sy)) else s_p2
        )
        v2p = (
            s_p3 if dist_sq(s_p3, (sx, sy)) > dist_sq(s_p4, (sx, sy)) else s_p4
        )
        ang1 = math.atan2(v1p[1] - sy, v1p[0] - sx)
        ang2 = math.atan2(v2p[1] - sy, v2p[0] - sx)
        return sx, sy, ang1, ang2

    def _get_line_shape_visuals(self, line, shape, reg, to_screen):
        center = reg.get_point(shape.center_idx)
        lp1, lp2 = reg.get_point(line.p1_idx), reg.get_point(line.p2_idx)
        dxL, dyL = lp2.x - lp1.x, lp2.y - lp1.y
        if math.hypot(dxL, dyL) < 1e-9:
            return None
        ux, uy = dxL / math.hypot(dxL, dyL), dyL / math.hypot(dxL, dyL)

        if isinstance(shape, Arc):
            sp = reg.get_point(shape.start_idx)
        else:
            sp = reg.get_point(shape.radius_pt_idx)
        radius = math.hypot(sp.x - center.x, sp.y - center.y)

        ix1, iy1 = center.x + radius * ux, center.y + radius * uy
        ix2, iy2 = center.x - radius * ux, center.y - radius * uy

        valid_points = []
        for ix, iy in [(ix1, iy1), (ix2, iy2)]:
            on_line = is_point_on_line_segment(
                (ix, iy), (lp1.x, lp1.y), (lp2.x, lp2.y)
            )
            on_arc = True
            if isinstance(shape, Arc):
                angle = math.atan2(iy - center.y, ix - center.x)
                on_arc = shape.is_angle_within_sweep(angle, reg)
            if on_line and on_arc:
                valid_points.append((ix, iy))

        if valid_points:
            best_pt = valid_points[0]
            if len(valid_points) > 1:
                lmx, lmy = (lp1.x + lp2.x) / 2, (lp1.y + lp2.y) / 2
                d1 = (best_pt[0] - lmx) ** 2 + (best_pt[1] - lmy) ** 2
                d2 = (valid_points[1][0] - lmx) ** 2 + (
                    valid_points[1][1] - lmy
                ) ** 2
                if d2 < d1:
                    best_pt = valid_points[1]
            sx, sy = to_screen(best_pt)
            return sx, sy, None, None

        sx, sy = to_screen((center.x, center.y))
        return sx, sy, None, None

    def _get_shape_shape_visuals(self, s1, s2, reg, to_screen):
        c1, c2 = reg.get_point(s1.center_idx), reg.get_point(s2.center_idx)
        r1 = math.hypot(
            reg.get_point(
                s1.start_idx if isinstance(s1, Arc) else s1.radius_pt_idx
            ).x
            - c1.x,
            reg.get_point(
                s1.start_idx if isinstance(s1, Arc) else s1.radius_pt_idx
            ).y
            - c1.y,
        )
        r2 = math.hypot(
            reg.get_point(
                s2.start_idx if isinstance(s2, Arc) else s2.radius_pt_idx
            ).x
            - c2.x,
            reg.get_point(
                s2.start_idx if isinstance(s2, Arc) else s2.radius_pt_idx
            ).y
            - c2.y,
        )

        intersections = get_circle_circle_intersections(
            (c1.x, c1.y), r1, (c2.x, c2.y), r2
        )
        if not intersections:
            return None

        valid_points = []
        for ix, iy in intersections:
            on_s1 = (
                s1.is_angle_within_sweep(math.atan2(iy - c1.y, ix - c1.x), reg)
                if isinstance(s1, Arc)
                else True
            )
            on_s2 = (
                s2.is_angle_within_sweep(math.atan2(iy - c2.y, ix - c2.x), reg)
                if isinstance(s2, Arc)
                else True
            )
            if on_s1 and on_s2:
                valid_points.append((ix, iy))

        if not valid_points:
            sx, sy = to_screen(intersections[0])
            return sx, sy, None, None

        best_pt = valid_points[0]
        if len(valid_points) > 1:
            m1 = s1.get_midpoint(reg) if isinstance(s1, Arc) else None
            m2 = s2.get_midpoint(reg) if isinstance(s2, Arc) else None
            if m1 and m2:
                d1 = (
                    (valid_points[0][0] - m1[0]) ** 2
                    + (valid_points[0][1] - m1[1]) ** 2
                    + (valid_points[0][0] - m2[0]) ** 2
                    + (valid_points[0][1] - m2[1]) ** 2
                )
                d2 = (
                    (valid_points[1][0] - m1[0]) ** 2
                    + (valid_points[1][1] - m1[1]) ** 2
                    + (valid_points[1][0] - m2[0]) ** 2
                    + (valid_points[1][1] - m2[1]) ** 2
                )
                if d2 < d1:
                    best_pt = valid_points[1]

        sx, sy = to_screen(best_pt)
        return sx, sy, None, None

    def is_hit(
        self,
        sx: float,
        sy: float,
        reg: EntityRegistry,
        to_screen: Callable[[Point], Point],
        element: Any,
        threshold: float,
    ) -> bool:
        data = self.get_visuals(reg, to_screen)
        if not data:
            return False

        cx, cy, ang1, ang2 = data

        if ang1 is not None and ang2 is not None:
            # Case 1: Line-Line (Angles are provided)
            # The marker is an arc with a dot. We hit-test the specific dot
            # location.
            visual_radius = 16.0  # Matches renderer.py
            diff = ang2 - ang1

            # Normalize angle difference to [-pi, pi]
            while diff <= -math.pi:
                diff += 2 * math.pi
            while diff > math.pi:
                diff -= 2 * math.pi

            mid_angle = ang1 + diff / 2
            # The dot is drawn at 0.6 * radius
            target_x = cx + math.cos(mid_angle) * visual_radius * 0.6
            target_y = cy + math.sin(mid_angle) * visual_radius * 0.6

            return math.hypot(sx - target_x, sy - target_y) < threshold
        else:
            # Case 2: Line-Arc / Arc-Arc (Box style)
            # The marker is a square box at the intersection/anchor.
            return math.hypot(sx - cx, sy - cy) < threshold

    def draw(
        self,
        ctx: cairo.Context,
        registry: EntityRegistry,
        to_screen: Callable[[Point], Point],
        is_selected: bool = False,
        is_hovered: bool = False,
        point_radius: float = 5.0,
    ) -> None:
        data = self.get_visuals(registry, to_screen)
        if not data:
            return

        sx, sy, ang1, ang2 = data

        ctx.save()
        ctx.set_line_width(1.5)

        if ang1 is not None and ang2 is not None:
            radius = 16.0
            diff = ang2 - ang1
            while diff <= -math.pi:
                diff += 2 * math.pi
            while diff > math.pi:
                diff -= 2 * math.pi

            ctx.new_sub_path()
            if diff > 0:
                ctx.arc(sx, sy, radius, ang1, ang2)
            else:
                ctx.arc_negative(sx, sy, radius, ang1, ang2)

            if is_selected:
                self._draw_selection_underlay(ctx)

            if self.status == ConstraintStatus.CONFLICTING:
                self._draw_conflict_underlay(ctx)

            self._set_color(ctx, is_hovered)
            ctx.stroke()

            # Dot
            mid = ang1 + diff / 2
            dx = sx + math.cos(mid) * radius * 0.6
            dy = sy + math.sin(mid) * radius * 0.6
            ctx.new_sub_path()
            ctx.arc(dx, dy, 2.0, 0, 2 * math.pi)
            ctx.fill()
        else:
            sz = 8.0
            ctx.new_sub_path()
            ctx.rectangle(sx - sz, sy - sz, sz * 2, sz * 2)

            if is_selected:
                self._draw_selection_underlay(ctx)

            if self.status == ConstraintStatus.CONFLICTING:
                self._draw_conflict_underlay(ctx)

            self._set_color(ctx, is_hovered)
            ctx.stroke()

        ctx.restore()

    def error(self, reg: EntityRegistry, params: ParameterContext) -> float:
        return super().error(reg)[0]

    def gradient(
        self, reg: EntityRegistry, params: ParameterContext
    ) -> dict[EntityID, list[Point]]:
        return super().gradient(reg)

    def to_dict(self) -> dict[str, Any]:
        return super().to_dict()

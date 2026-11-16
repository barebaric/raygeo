from __future__ import annotations

import math
from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

import cairo

from raygeo.geo.shape.line import get_line_closest_point
from raygeo.geo.types import Point
from raygeo.sketch.constraints.tangent import (
    TangentConstraint as _CoreTangentConstraint,
)
from raygeo.sketch.entities import Line

from .base import Constraint, ConstraintStatus

if TYPE_CHECKING:
    from raygeo.sketch.entities import Arc, Circle, Entity, Line
    from raygeo.sketch.model.protocols import (
        ParameterContext,
        Sketch,
        SketchSelection,
    )
    from raygeo.sketch.registry import EntityRegistry
    from raygeo.sketch.types import EntityID


class TangentConstraint(_CoreTangentConstraint, Constraint):
    """
    Enforces tangency between a Line and an Arc/Circle.
    Logic: Distance from shape center to Line equals shape Radius.
    """

    def __new__(cls, line_id, shape_id, user_visible=True):
        return super().__new__(cls, line_id, shape_id, user_visible)

    def __init__(
        self, line_id: EntityID, shape_id: EntityID, user_visible: bool = True
    ):
        self.line_id: EntityID = line_id
        self.shape_id: EntityID = shape_id

    @classmethod
    def get_type_key(cls) -> str:
        return "tangent"

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
        """One operand must be a Line, the other radius-bearing."""
        if len(entities) != 2:
            return False
        e1, e2 = entities
        if isinstance(e1, Line):
            return e2.is_radius_entity()
        return isinstance(e2, Line) and e1.is_radius_entity()

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Tangent")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: EntityRegistry) -> str:
        """Returns subtitle describing constrained entities."""
        line = registry.get_entity(self.line_id)
        shape = registry.get_entity(self.shape_id)
        if isinstance(line, Line) and isinstance(shape, (Arc, Circle)):
            center = registry.get_point(shape.center_idx)
            if center:
                return _("Line to {} at {}").format(
                    type(shape).__name__,
                    self._format_coord(center.x, center.y),
                )
        return ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> TangentConstraint:
        return cls(
            line_id=data["line_id"],
            shape_id=data["shape_id"],
            user_visible=data.get("user_visible", True),
        )

    def is_hit(
        self,
        sx: float,
        sy: float,
        reg: EntityRegistry,
        to_screen: Callable[[Point], Point],
        element: Any,
        threshold: float,
    ) -> bool:
        line = reg.get_entity(self.line_id)
        shape = reg.get_entity(self.shape_id)

        if not (
            line
            and shape
            and isinstance(line, Line)
            and isinstance(shape, (Arc, Circle))
        ):
            return False

        p1 = reg.get_point(line.p1_idx)
        p2 = reg.get_point(line.p2_idx)
        center = reg.get_point(shape.center_idx)

        if not (p1 and p2 and center):
            return False

        # Find closest point on infinite line from center
        tangent_mx, tangent_my = get_line_closest_point(
            (p1.x, p1.y), (p2.x, p2.y), center.x, center.y
        )

        # Convert to Screen Space first
        sx_tangent, sy_tangent = to_screen((tangent_mx, tangent_my))
        sx_center, sy_center = to_screen((center.x, center.y))

        # Calculate angle in screen space
        angle = math.atan2(sy_tangent - sy_center, sx_tangent - sx_center)

        # Apply offset in screen pixels
        offset = 15.0
        symbol_sx = sx_tangent + offset * math.cos(angle)
        symbol_sy = sy_tangent + offset * math.sin(angle)

        return math.hypot(sx - symbol_sx, sy - symbol_sy) < threshold

    def draw(
        self,
        ctx: cairo.Context,
        registry: EntityRegistry,
        to_screen: Callable[[Point], Point],
        is_selected: bool = False,
        is_hovered: bool = False,
        point_radius: float = 5.0,
    ) -> None:
        line = registry.get_entity(self.line_id)
        shape = registry.get_entity(self.shape_id)

        if not (
            line
            and shape
            and isinstance(line, Line)
            and isinstance(shape, (Arc, Circle))
        ):
            return

        p1 = registry.get_point(line.p1_idx)
        p2 = registry.get_point(line.p2_idx)
        center = registry.get_point(shape.center_idx)

        # Find closest point on infinite line from center (in model space)
        tangent_mx, tangent_my = get_line_closest_point(
            (p1.x, p1.y), (p2.x, p2.y), center.x, center.y
        )

        sx_tangent, sy_tangent = to_screen((tangent_mx, tangent_my))
        sx_center, sy_center = to_screen((center.x, center.y))

        angle = math.atan2(sy_tangent - sy_center, sx_tangent - sx_center)

        offset = 15.0
        sx = sx_tangent + offset * math.cos(angle)
        sy = sy_tangent + offset * math.sin(angle)

        ctx.save()
        ctx.set_line_width(1.5)

        ctx.translate(sx, sy)
        ctx.rotate(angle + math.pi / 2.0)

        radius = 6.0

        ctx.new_sub_path()
        ctx.arc(
            0,
            -radius,
            radius,
            math.pi / 2 - math.pi / 3,
            math.pi / 2 + math.pi / 3,
        )
        ctx.move_to(-radius * 1.2, 0)
        ctx.line_to(radius * 1.2, 0)

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

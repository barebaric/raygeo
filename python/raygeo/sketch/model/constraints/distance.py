from __future__ import annotations

from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

import cairo

try:
    from gi.repository import PangoCairo
except ImportError:
    PangoCairo = None

from raygeo.geo.shape.line import get_line_segment_closest_point
from raygeo.geo.types import Point
from raygeo.sketch.constraints.distance import (
    DistanceConstraint as _CoreDistanceConstraint,
)
from raygeo.sketch.entities import Line
from raygeo.sketch.model.text import create_text_layout

from .base import Constraint, ConstraintStatus

if TYPE_CHECKING:
    from raygeo.sketch.entities import Entity, Line
    from raygeo.sketch.model.protocols import (
        ParameterContext,
        Sketch,
        SketchSelection,
    )
    from raygeo.sketch.types import EntityID


class DistanceConstraint(_CoreDistanceConstraint, Constraint):
    """Enforces distance between two points."""

    def __new__(cls, p1, p2, value, expression=None, user_visible=True):
        obj = super().__new__(cls, p1, p2, 0.0, user_visible)
        obj.seed_reference_attrs()
        return obj

    def __init__(
        self,
        p1: EntityID,
        p2: EntityID,
        value: str | float,
        expression: str | None = None,
        user_visible: bool = True,
    ):
        self.p1: EntityID = p1
        self.p2: EntityID = p2

        if expression is not None:
            self.expression = expression
            self.value = float(value)
        elif isinstance(value, str):
            self.expression = value
            self.value = 0.0
        else:
            self.expression = None
            self.value = float(value)

    @classmethod
    def get_type_key(cls) -> str:
        return "dist"

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        if len(selection.point_ids) == 2 and not selection.entity_ids:
            return True
        if selection.point_ids:
            return False
        entities = selection.resolve_entities(
            sketch.registry if sketch else None
        )
        if entities is None or len(entities) != 1:
            return False
        return cls.applies_to_entities(entities)

    @classmethod
    def applies_to_entities(cls, entities: Sequence[Entity]) -> bool:
        """The operand must be a Line."""
        return len(entities) == 1 and isinstance(entities[0], Line)

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Distance")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return f"{self.get_type_name()} {self._format_value()}"

    def get_subtitle(self, registry: Any) -> str:
        """Returns a subtitle describing the constrained points."""
        p1 = registry.get_point(self.p1)
        p2 = registry.get_point(self.p2)
        if p1 and p2:
            return _("From {} to {}").format(
                self._format_coord(p1.x, p1.y),
                self._format_coord(p2.x, p2.y),
            )
        return ""

    def get_edit_subtitle(self) -> str:
        return _("Enter length or expression.")

    def targets_segment(
        self, p1: EntityID, p2: EntityID, entity_id: EntityID | None
    ) -> bool:
        return {self.p1, self.p2} == {p1, p2}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> DistanceConstraint:
        return cls(
            p1=data["p1"],
            p2=data["p2"],
            value=data["value"],
            expression=data.get("expression"),
            user_visible=data.get("user_visible", True),
        )

    def get_label_pos(
        self,
        reg: Any,
        to_screen: Callable[[Point], Point],
        element: Any,
    ):
        """Calculates screen position for distance constraint label."""
        p1 = reg.get_point(self.p1)
        p2 = reg.get_point(self.p2)
        if not (p1 and p2):
            return None

        s1 = to_screen((p1.x, p1.y))
        s2 = to_screen((p2.x, p2.y))
        mx = (s1[0] + s2[0]) / 2
        my = (s1[1] + s2[1]) / 2

        return mx, my

    def is_hit(
        self,
        sx: float,
        sy: float,
        reg: Any,
        to_screen: Callable[[Point], Point],
        element: Any,
        threshold: float,
    ) -> bool:
        p1 = reg.get_point(self.p1)
        p2 = reg.get_point(self.p2)

        has_geometry = False
        if p1 and p2:
            entities = reg.entities or []
            for entity in entities:
                if isinstance(entity, Line) and {
                    entity.p1_idx,
                    entity.p2_idx,
                } == {
                    self.p1,
                    self.p2,
                }:
                    has_geometry = True
                    break

            if not has_geometry:
                s1 = to_screen((p1.x, p1.y))
                s2 = to_screen((p2.x, p2.y))

                _, _, dist_sq = get_line_segment_closest_point(s1, s2, sx, sy)

                if dist_sq < threshold**2:
                    return True

        pos_data = self.get_label_pos(reg, to_screen, element)
        if pos_data:
            label_sx, label_sy = pos_data

            label_width = 20.0
            label_height = 20.0
            half_w = label_width / 2.0
            half_h = label_height / 2.0

            x_min = label_sx - half_w - 4.0
            x_max = label_sx + half_w + 4.0
            y_min = label_sy - half_h - 4.0
            y_max = label_sy + half_h + 4.0

            return x_min <= sx <= x_max and y_min <= sy <= y_max
        return False

    def draw(
        self,
        ctx: cairo.Context,
        registry: Any,
        to_screen: Callable[[Point], Point],
        is_selected: bool = False,
        is_hovered: bool = False,
        point_radius: float = 5.0,
    ) -> None:
        try:
            p1 = registry.get_point(self.p1)
            p2 = registry.get_point(self.p2)
        except IndexError:
            return

        s1 = to_screen((p1.x, p1.y))
        s2 = to_screen((p2.x, p2.y))
        mx, my = (s1[0] + s2[0]) / 2, (s1[1] + s2[1]) / 2

        label = self._format_value()
        layout = create_text_layout(ctx, label)
        ext, _ = layout.get_pixel_extents()

        ctx.save()
        # Set background color based on selection, hover, and status
        if is_selected:
            ctx.set_source_rgba(0.2, 0.6, 1.0, 0.4)  # Blue selection
        elif is_hovered:
            ctx.set_source_rgba(1.0, 0.95, 0.85, 0.9)  # Light yellow hover
        elif self.status == ConstraintStatus.CONFLICTING:
            ctx.set_source_rgba(1.0, 0.6, 0.6, 0.9)  # Red background
        elif self.status == ConstraintStatus.ERROR:
            ctx.set_source_rgba(1.0, 0.8, 0.8, 0.9)  # Light red background
        elif self.status == ConstraintStatus.EXPRESSION_BASED:
            ctx.set_source_rgba(1.0, 0.9, 0.7, 0.9)  # Light orange background
        else:  # VALID
            ctx.set_source_rgba(1, 1, 1, 0.8)  # Default white background

        # Draw label background
        bg_x = mx - ext.width / 2 - 4
        bg_y = my - ext.height / 2 - 4
        ctx.rectangle(bg_x, bg_y, ext.width + 8, ext.height + 8)
        ctx.fill()
        ctx.new_path()

        # Set text color based on status
        if self.status in (
            ConstraintStatus.ERROR,
            ConstraintStatus.CONFLICTING,
        ):
            ctx.set_source_rgb(0.8, 0.0, 0.0)  # Red text for error/conflict
        else:
            ctx.set_source_rgb(0, 0, 0.5)  # Dark blue otherwise

        ctx.move_to(
            mx - ext.width / 2 - ext.x,
            my - ext.height / 2 - ext.y,
        )
        PangoCairo.show_layout(ctx, layout)
        ctx.new_path()

        # Draw Dash Line - only if no solid line entity connects these points
        has_geometry = False
        entities = registry.entities or []
        for entity in entities:
            if isinstance(entity, Line) and {entity.p1_idx, entity.p2_idx} == {
                self.p1,
                self.p2,
            }:
                has_geometry = True
                break

        if not has_geometry:
            ctx.set_line_width(1)
            ctx.set_dash([4, 4])
            ctx.move_to(s1[0], s1[1])
            ctx.line_to(s2[0], s2[1])

            if self.status == ConstraintStatus.CONFLICTING:
                self._draw_conflict_underlay(ctx)

            self._set_color(ctx, is_hovered)
            ctx.stroke()

        ctx.restore()

    def get_referenced_point_ids(self) -> set[EntityID]:
        return Constraint.get_referenced_point_ids(self)

    def get_referenced_entity_ids(self) -> set[EntityID]:
        return Constraint.get_referenced_entity_ids(self)

    def error(self, reg: Any, params: ParameterContext) -> float:
        return super().error(reg)[0]

    def gradient(
        self, reg: Any, params: ParameterContext
    ) -> dict[EntityID, list[Point]]:
        return super().gradient(reg)

    def to_dict(self) -> dict[str, Any]:
        return super().to_dict()

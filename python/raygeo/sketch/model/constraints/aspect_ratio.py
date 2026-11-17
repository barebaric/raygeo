from __future__ import annotations

import math
from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

import cairo

from raygeo.geo.types import Point
from raygeo.sketch.constraints.aspect_ratio import (
    AspectRatioConstraint as _CoreAspectRatioConstraint,
)
from raygeo.sketch.entities import Line

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


class AspectRatioConstraint(_CoreAspectRatioConstraint, Constraint):
    """Enforces that distance(p1, p2) / distance(p3, p4) equals ratio."""

    def __new__(cls, p1, p2, p3, p4, ratio, user_visible=True):
        obj = super().__new__(cls, p1, p2, p3, p4, ratio, user_visible)
        obj.seed_reference_attrs()
        return obj

    def __init__(
        self,
        p1: EntityID,
        p2: EntityID,
        p3: EntityID,
        p4: EntityID,
        ratio: float,
        user_visible: bool = True,
    ):
        self.p1: EntityID = p1
        self.p2: EntityID = p2
        self.p3: EntityID = p3
        self.p4: EntityID = p4
        self.ratio = ratio

    @classmethod
    def get_type_key(cls) -> str:
        return "aspect_ratio"

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
        """Both operands must be Lines."""
        return len(entities) == 2 and all(
            isinstance(e, Line) for e in entities
        )

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Aspect Ratio")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return f"{self.get_type_name()} {self.ratio:.2f}"

    def get_subtitle(self, registry: EntityRegistry) -> str:
        """Returns subtitle describing constrained segments."""
        p1 = registry.get_point(self.p1)
        p2 = registry.get_point(self.p2)
        if p1 and p2:
            return _("From {} to {}").format(
                self._format_coord(p1.x, p1.y),
                self._format_coord(p2.x, p2.y),
            )
        return ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> AspectRatioConstraint:
        return cls(
            p1=data["p1"],
            p2=data["p2"],
            p3=data["p3"],
            p4=data["p4"],
            ratio=data["ratio"],
            user_visible=data.get("user_visible", True),
        )

    def _get_icon_pos(
        self,
        reg: EntityRegistry,
        to_screen: Callable[[Point], Point],
    ) -> Point | None:
        """Calculates the screen position of the constraint icon."""
        p_ids1 = {self.p1, self.p2}
        p_ids2 = {self.p3, self.p4}
        junction_ids = p_ids1.intersection(p_ids2)

        if len(junction_ids) == 1:
            # Position near the common junction point
            junction_id = junction_ids.pop()
            p_junc = reg.get_point(junction_id)

            p_other1_id = self.p1 if self.p2 == junction_id else self.p2
            p_other2_id = self.p3 if self.p4 == junction_id else self.p4
            p_other1 = reg.get_point(p_other1_id)
            p_other2 = reg.get_point(p_other2_id)

            # Calculate vectors in screen space for a consistent offset
            s_junc = to_screen((p_junc.x, p_junc.y))
            s_other1 = to_screen((p_other1.x, p_other1.y))
            s_other2 = to_screen((p_other2.x, p_other2.y))

            sv1 = (s_other1[0] - s_junc[0], s_other1[1] - s_junc[1])
            sv2 = (s_other2[0] - s_junc[0], s_other2[1] - s_junc[1])
            slen1 = math.hypot(sv1[0], sv1[1])
            slen2 = math.hypot(sv2[0], sv2[1])

            if slen1 < 1e-9 or slen2 < 1e-9:
                return s_junc

            su1 = (sv1[0] / slen1, sv1[1] / slen1)
            su2 = (sv2[0] / slen2, sv2[1] / slen2)

            # External angle bisector direction in screen space
            s_bisector = (-(su1[0] + su2[0]), -(su1[1] + su2[1]))
            len_sb = math.hypot(s_bisector[0], s_bisector[1])

            if len_sb < 1e-9:
                # Fallback for parallel/opposite vectors
                s_bisector = (-su1[1], su1[0])
                len_sb = 1.0

            su_bisector = (s_bisector[0] / len_sb, s_bisector[1] / len_sb)

            offset = 18.0  # screen pixels
            return (
                s_junc[0] + offset * su_bisector[0],
                s_junc[1] + offset * su_bisector[1],
            )
        else:
            # Fallback for non-adjoining lines: position at center of midpoints
            p1 = reg.get_point(self.p1)
            p2 = reg.get_point(self.p2)
            p3 = reg.get_point(self.p3)
            p4 = reg.get_point(self.p4)
            m1_x, m1_y = (p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0
            m2_x, m2_y = (p3.x + p4.x) / 2.0, (p3.y + p4.y) / 2.0
            center_mx, center_my = (m1_x + m2_x) / 2.0, (m1_y + m2_y) / 2.0
            return to_screen((center_mx, center_my))

    def is_hit(
        self,
        sx: float,
        sy: float,
        reg: EntityRegistry,
        to_screen: Callable[[Point], Point],
        element: Any,
        threshold: float,
    ) -> bool:
        icon_pos = self._get_icon_pos(reg, to_screen)
        if icon_pos:
            cx, cy = icon_pos
            return math.hypot(sx - cx, sy - cy) < threshold
        return False

    def draw(
        self,
        ctx: cairo.Context,
        registry: EntityRegistry,
        to_screen: Callable[[Point], Point],
        is_selected: bool = False,
        is_hovered: bool = False,
        point_radius: float = 5.0,
    ) -> None:
        icon_pos = self._get_icon_pos(registry, to_screen)
        if not icon_pos:
            return

        cx, cy = icon_pos

        ctx.save()

        icon_size = 16.0

        # Draw a circular underlay for selection, similar to other icons
        if is_selected:
            ctx.set_source_rgba(0.2, 0.6, 1.0, 0.4)
            # Use a slightly larger radius for the glow effect
            ctx.arc(cx, cy, icon_size / 2.0 + 4.0, 0, 2 * math.pi)
            ctx.fill()

        if self.status == ConstraintStatus.CONFLICTING:
            ctx.set_source_rgba(1.0, 0.2, 0.2, 0.5)
            ctx.arc(cx, cy, icon_size / 2.0 + 4.0, 0, 2 * math.pi)
            ctx.fill()

        # Translate to the icon's anchor point for easier drawing
        ctx.translate(cx, cy)

        hs = icon_size / 2.0

        ctx.set_line_width(2.0)
        ctx.set_line_cap(cairo.LINE_CAP_ROUND)
        ctx.set_line_join(cairo.LINE_JOIN_ROUND)

        ctx.new_path()

        # Top-right corner bracket (L-shape pointing into the corner)
        ctx.move_to(hs * 0.4, hs)
        ctx.line_to(hs, hs)
        ctx.line_to(hs, hs * 0.4)

        # Bottom-left corner bracket
        ctx.move_to(-hs * 0.4, -hs)
        ctx.line_to(-hs, -hs)
        ctx.line_to(-hs, -hs * 0.4)

        # Set color and draw the icon
        self._set_color(ctx, is_hovered)
        ctx.stroke()

        ctx.restore()

    def get_referenced_point_ids(self) -> set[EntityID]:
        return Constraint.get_referenced_point_ids(self)

    def get_referenced_entity_ids(self) -> set[EntityID]:
        return Constraint.get_referenced_entity_ids(self)

    def error(self, reg: EntityRegistry, params: ParameterContext) -> float:
        return super().error(reg)[0]

    def gradient(
        self, reg: EntityRegistry, params: ParameterContext
    ) -> dict[EntityID, list[Point]]:
        return super().gradient(reg)

    def to_dict(self) -> dict[str, Any]:
        return super().to_dict()

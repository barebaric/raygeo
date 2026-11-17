from __future__ import annotations

import math
from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

import cairo
from gi.repository import PangoCairo

from raygeo.geo.shape.arc import normalize_angle
from raygeo.geo.shape.line import get_line_line_intersection
from raygeo.geo.types import Point
from raygeo.sketch.constraints.angle import (
    AngleConstraint as _CoreAngleConstraint,
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

ANGLE_WEIGHT = 10.0
ARC_RADIUS = 35.0
LABEL_RADIUS = ARC_RADIUS - 10.0


def _get_far_point(ix, iy, p1, p2):
    d1 = (p1.x - ix) ** 2 + (p1.y - iy) ** 2
    d2 = (p2.x - ix) ** 2 + (p2.y - iy) ** 2
    return p1 if d1 > d2 else p2


ARC_RADIUS = 35.0
LABEL_RADIUS = ARC_RADIUS - 10.0


class AngleConstraint(_CoreAngleConstraint, Constraint):
    """
    Enforces a specific angle between two lines.

    e1 is the anchor line, e2 is the other line.
    The angle is measured CW from anchor direction to other direction,
    using directions pointing away from the lines' intersection toward
    the stored far points.
    """

    def __new__(
        cls,
        e1_id,
        e2_id,
        value,
        expression=None,
        user_visible=True,
        e1_far_idx=None,
        e2_far_idx=None,
    ):
        obj = super().__new__(
            cls, e1_id, e2_id, 0.0, user_visible, e1_far_idx, e2_far_idx
        )
        obj.seed_reference_attrs()
        return obj

    def __init__(
        self,
        e1_id: EntityID,
        e2_id: EntityID,
        value: str | float,
        expression: str | None = None,
        user_visible: bool = True,
        e1_far_idx: EntityID | None = None,
        e2_far_idx: EntityID | None = None,
    ):
        self.e1_id: EntityID = e1_id
        self.e2_id: EntityID = e2_id
        self.e1_far_idx: EntityID | None = e1_far_idx
        self.e2_far_idx: EntityID | None = e2_far_idx

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
        return "angle"

    def get_referenced_entity_ids(self) -> set[EntityID]:
        return {self.e1_id, self.e2_id}

    def get_referenced_point_ids(self) -> set[EntityID]:
        ids: set[EntityID] = set()
        if self.e1_far_idx is not None:
            ids.add(self.e1_far_idx)
        if self.e2_far_idx is not None:
            ids.add(self.e2_far_idx)
        return ids

    def is_mirror_compatible(self) -> bool:
        # An expression-based angle encodes a user intent (e.g. "width/2")
        # that does not auto-flip under reflection, so it cannot be
        # preserved. A plain numeric angle can be negated — see mirror().
        return self.expression is None

    def mirror(self, axis) -> None:
        # Reflection reverses the signed (CW) angle. Negate the numeric
        # value so the constraint stays satisfied with the mirrored lines.
        if self.expression is not None:
            return  # should have been dropped via is_mirror_compatible
        self.value = -self.value

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
        return _("Angle")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return f"{self.get_type_name()} {self._format_value()}°"

    def get_subtitle(self, registry: Any) -> str:
        """Returns subtitle describing constrained entities."""
        e1 = registry.get_entity(self.e1_id)
        e2 = registry.get_entity(self.e2_id)
        if isinstance(e1, Line) and isinstance(e2, Line):
            return _("Between two lines")
        return ""

    def get_edit_subtitle(self) -> str:
        return _("Enter angle in degrees or expression.")

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> AngleConstraint:
        return cls(
            e1_id=data["e1_id"],
            e2_id=data["e2_id"],
            value=data["value"],
            expression=data.get("expression"),
            user_visible=data.get("user_visible", True),
            e1_far_idx=data.get("e1_far_idx"),
            e2_far_idx=data.get("e2_far_idx"),
        )

    def _get_line_params(
        self, reg: Any
    ) -> tuple[Line, Line, Any, Any, Any, Any] | None:
        e1 = reg.get_entity(self.e1_id)
        e2 = reg.get_entity(self.e2_id)
        if not (isinstance(e1, Line) and isinstance(e2, Line)):
            return None

        p1 = reg.get_point(e1.p1_idx)
        p2 = reg.get_point(e1.p2_idx)
        p3 = reg.get_point(e2.p1_idx)
        p4 = reg.get_point(e2.p2_idx)

        if not (p1 and p2 and p3 and p4):
            return None

        return e1, e2, p1, p2, p3, p4

    def _get_far_points(
        self, e1: Line, e2: Line, p1, p2, p3, p4, ix: float, iy: float
    ):
        if self.e1_far_idx == e1.p1_idx:
            far1 = p1
        elif self.e1_far_idx == e1.p2_idx:
            far1 = p2
        else:
            far1 = _get_far_point(ix, iy, p1, p2)

        if self.e2_far_idx == e2.p1_idx:
            far2 = p3
        elif self.e2_far_idx == e2.p2_idx:
            far2 = p4
        else:
            far2 = _get_far_point(ix, iy, p3, p4)

        return far1, far2

    def get_visuals(
        self,
        reg: Any,
        to_screen: Callable[[Point], Point],
    ):
        result = self._get_line_params(reg)
        if result is None:
            return None

        e1, e2, p1, p2, p3, p4 = result

        intersection = get_line_line_intersection(
            (p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y), (p4.x, p4.y)
        )

        if intersection is None:
            mx1 = (p1.x + p2.x) / 2
            my1 = (p1.y + p2.y) / 2
            mx2 = (p3.x + p4.x) / 2
            my2 = (p3.y + p4.y) / 2
            intersection = ((mx1 + mx2) / 2, (my1 + my2) / 2)

        ix, iy = intersection
        sx, sy = to_screen((ix, iy))

        far1, far2 = self._get_far_points(e1, e2, p1, p2, p3, p4, ix, iy)

        far1_screen = to_screen((far1.x, far1.y))
        far2_screen = to_screen((far2.x, far2.y))

        anchor_ang = math.atan2(far1_screen[1] - sy, far1_screen[0] - sx)
        other_ang = math.atan2(far2_screen[1] - sy, far2_screen[0] - sx)

        return sx, sy, anchor_ang, other_ang

    def get_label_pos(
        self,
        reg: Any,
        to_screen: Callable[[Point], Point],
        element: Any,
    ):
        visuals = self.get_visuals(reg, to_screen)
        if visuals is None:
            return None

        sx, sy, anchor_ang, other_ang = visuals

        cw_diff = normalize_angle(anchor_ang - other_ang)

        mid_angle = other_ang + cw_diff / 2
        radius = 28.0
        label_x = sx + math.cos(mid_angle) * radius
        label_y = sy + math.sin(mid_angle) * radius

        return label_x, label_y

    def is_hit(
        self,
        sx: float,
        sy: float,
        reg: Any,
        to_screen: Callable[[Point], Point],
        element: Any,
        threshold: float,
    ) -> bool:
        visuals = self.get_visuals(reg, to_screen)
        if visuals is None:
            return False

        cx, cy, anchor_ang, other_ang = visuals

        dist = math.hypot(sx - cx, sy - cy)
        if abs(dist - ARC_RADIUS) > threshold:
            return False

        click_ang = math.atan2(sy - cy, sx - cx)

        ccw_diff = normalize_angle(other_ang - anchor_ang)

        click_from_anchor = normalize_angle(click_ang - anchor_ang)

        return click_from_anchor <= ccw_diff

    def draw(
        self,
        ctx: cairo.Context,
        registry: Any,
        to_screen: Callable[[Point], Point],
        is_selected: bool = False,
        is_hovered: bool = False,
        point_radius: float = 5.0,
    ) -> None:
        visuals = self.get_visuals(registry, to_screen)
        if visuals is None:
            return

        sx, sy, anchor_ang, other_ang = visuals

        ctx.save()
        ctx.set_line_width(1.5)

        ctx.new_sub_path()
        ctx.arc(sx, sy, ARC_RADIUS, anchor_ang, other_ang)

        if is_selected:
            self._draw_selection_underlay(ctx)

        if self.status == ConstraintStatus.CONFLICTING:
            self._draw_conflict_underlay(ctx)

        self._set_color(ctx, is_hovered)
        ctx.stroke()

        label = self._format_value() + "°"
        layout = create_text_layout(ctx, label)
        ext, _ = layout.get_pixel_extents()

        ccw_diff = normalize_angle(other_ang - anchor_ang)

        mid = anchor_ang + ccw_diff / 2
        label_x = sx + math.cos(mid) * LABEL_RADIUS
        label_y = sy + math.sin(mid) * LABEL_RADIUS

        if is_selected:
            ctx.set_source_rgba(0.2, 0.6, 1.0, 0.4)
        elif is_hovered:
            ctx.set_source_rgba(1.0, 0.95, 0.85, 0.9)
        elif self.status == ConstraintStatus.CONFLICTING:
            ctx.set_source_rgba(1.0, 0.6, 0.6, 0.9)
        elif self.status == ConstraintStatus.ERROR:
            ctx.set_source_rgba(1.0, 0.8, 0.8, 0.9)
        elif self.status == ConstraintStatus.EXPRESSION_BASED:
            ctx.set_source_rgba(1.0, 0.9, 0.7, 0.9)
        else:
            ctx.set_source_rgba(1, 1, 1, 0.8)

        bg_x = label_x - ext.width / 2 - 4
        bg_y = label_y - ext.height / 2 - 4
        ctx.rectangle(bg_x, bg_y, ext.width + 8, ext.height + 8)
        ctx.fill()
        ctx.new_path()

        if self.status in (
            ConstraintStatus.ERROR,
            ConstraintStatus.CONFLICTING,
        ):
            ctx.set_source_rgb(0.8, 0.0, 0.0)
        else:
            ctx.set_source_rgb(0, 0, 0.5)

        ctx.move_to(
            label_x - ext.width / 2 - ext.x,
            label_y - ext.height / 2 - ext.y,
        )
        PangoCairo.show_layout(ctx, layout)
        ctx.new_path()

        ctx.restore()

    def error(self, reg: Any, params: ParameterContext) -> float:
        return super().error(reg)[0]

    def gradient(
        self, reg: Any, params: ParameterContext
    ) -> dict[EntityID, list[Point]]:
        return super().gradient(reg)

    def to_dict(self) -> dict[str, Any]:
        return super().to_dict()

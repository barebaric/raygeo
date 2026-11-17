from __future__ import annotations

import math
from collections.abc import Callable
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

import cairo

from raygeo.geo.types import Point
from raygeo.sketch.constraints.coincident import (
    CoincidentConstraint as _CoreCoincidentConstraint,
)

from .base import Constraint, ConstraintStatus

if TYPE_CHECKING:
    from raygeo.sketch.model.protocols import (
        ParameterContext,
        Sketch,
        SketchSelection,
    )
    from raygeo.sketch.types import EntityID


class CoincidentConstraint(_CoreCoincidentConstraint, Constraint):
    """Enforces two points are at the same location."""

    def __new__(cls, p1, p2, user_visible=True):
        obj = super().__new__(cls, p1, p2, user_visible)
        obj.seed_reference_attrs()
        return obj

    def __init__(self, p1: EntityID, p2: EntityID, user_visible: bool = True):
        self.p1: EntityID = p1
        self.p2: EntityID = p2

    @classmethod
    def get_type_key(cls) -> str:
        return "coincident"

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        return len(selection.point_ids) == 2 and not selection.entity_ids

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Coincident")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: Any) -> str:
        """Returns a human-readable subtitle describing constrained points."""
        p1 = registry.get_point(self.p1)
        if p1:
            return _("At {}").format(self._format_coord(p1.x, p1.y))
        return ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> CoincidentConstraint:
        return cls(
            p1=data["p1"],
            p2=data["p2"],
            user_visible=data.get("user_visible", True),
        )

    def is_hit(
        self,
        sx: float,
        sy: float,
        reg: Any,
        to_screen: Callable[[Point], Point],
        element: Any,
        threshold: float,
    ) -> bool:
        origin_id = getattr(element.sketch, "origin_id", -1)
        pid_to_check = self.p1
        if self.p1 == origin_id and origin_id != -1:
            pid_to_check = self.p2

        pt_to_check = reg.get_point(pid_to_check)
        if pt_to_check:
            s_pt = to_screen((pt_to_check.x, pt_to_check.y))
            return math.hypot(sx - s_pt[0], sy - s_pt[1]) < threshold
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
        # Determine which point to draw on. Prefer the non-origin point if
        # one is the origin, to avoid clutter on the origin.
        # We don't have access to sketch.origin_id directly here without the
        # sketch object, but usually point 0 is origin.
        # Assuming p1 or p2 exists in registry.
        try:
            p = registry.get_point(self.p1)
            p2 = registry.get_point(self.p2)
        except IndexError:
            return

        # Heuristic: if p1 is fixed (likely origin), draw on p2.
        if p.fixed and not p2.fixed:
            p = p2

        sx, sy = to_screen((p.x, p.y))

        ctx.save()
        ctx.set_line_width(1.5)

        radius = point_radius + 4
        ctx.new_sub_path()
        ctx.arc(sx, sy, radius, 0, 2 * math.pi)

        if is_selected:
            self._draw_selection_underlay(ctx)

        if self.status == ConstraintStatus.CONFLICTING:
            self._draw_conflict_underlay(ctx)

        self._set_color(ctx, is_hovered)
        ctx.stroke()
        ctx.restore()

    def get_draggable_point(self) -> EntityID:
        """Returns p1 as the draggable point for coincident constraints."""
        return self.p1

    def get_referenced_point_ids(self) -> set[EntityID]:
        return Constraint.get_referenced_point_ids(self)

    def get_referenced_entity_ids(self) -> set[EntityID]:
        return Constraint.get_referenced_entity_ids(self)

    def error(self, reg: Any, params: ParameterContext) -> Point:
        err = super().error(reg)
        return (err[0], err[1])

    def gradient(
        self, reg: Any, params: ParameterContext
    ) -> dict[EntityID, list[Point]]:
        return super().gradient(reg)

    def to_dict(self) -> dict[str, Any]:
        return super().to_dict()

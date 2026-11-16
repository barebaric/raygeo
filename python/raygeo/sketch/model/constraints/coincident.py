from __future__ import annotations

import math
from collections.abc import Callable
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

from raygeo.geo.types import Point
from raygeo.sketch.constraints.coincident import (
    CoincidentConstraint as _CoreCoincidentConstraint,
)

from .base import Constraint

if TYPE_CHECKING:
    from raygeo.sketch.model.protocols import (
        ParameterContext,
        Sketch,
        SketchSelection,
    )
    from raygeo.sketch.registry import EntityRegistry
    from raygeo.sketch.types import EntityID


class CoincidentConstraint(_CoreCoincidentConstraint, Constraint):
    """Enforces two points are at the same location."""

    def __new__(cls, p1, p2, user_visible=True):
        return super().__new__(cls, p1, p2, user_visible)

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

    def get_subtitle(self, registry: EntityRegistry) -> str:
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
        reg: EntityRegistry,
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

    def get_draggable_point(self) -> EntityID:
        """Returns p1 as the draggable point for coincident constraints."""
        return self.p1

    def error(self, reg: EntityRegistry, params: ParameterContext) -> Point:
        err = super().error(reg)
        return Point(err[0], err[1])

    def gradient(
        self, reg: EntityRegistry, params: ParameterContext
    ) -> dict[EntityID, list[Point]]:
        return super().gradient(reg)

    def to_dict(self) -> dict[str, Any]:
        return super().to_dict()

    def draw(
        self,
        ctx: Any,
        registry: EntityRegistry,
        to_screen: Callable[[Point], Point],
        is_selected: bool = False,
        is_hovered: bool = False,
        point_radius: float = 5.0,
    ) -> None:
        return super().draw(
            ctx,
            registry,
            to_screen,
            is_selected,
            is_hovered,
            point_radius,
            status=int(self.status),
        )

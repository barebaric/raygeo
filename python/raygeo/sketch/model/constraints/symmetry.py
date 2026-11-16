from __future__ import annotations

import math
from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

from raygeo.geo.types import Point
from raygeo.sketch.constraints.symmetry import (
    SymmetryConstraint as _CoreSymmetryConstraint,
)
from raygeo.sketch.entities import Line

from .base import Constraint

if TYPE_CHECKING:
    from raygeo.sketch.entities import Entity
    from raygeo.sketch.model.protocols import (
        ParameterContext,
        Sketch,
        SketchSelection,
    )
    from raygeo.sketch.registry import EntityRegistry
    from raygeo.sketch.types import EntityID


class SymmetryConstraint(_CoreSymmetryConstraint, Constraint):
    """
    Enforces symmetry between two points (p1, p2) with respect to:
    1. A Center Point (Point Symmetry)
    2. An Axis Line (Line Symmetry)
    """

    def __new__(cls, p1, p2, center=None, axis=None, user_visible=True):
        return super().__new__(cls, p1, p2, center, axis, user_visible)

    def __init__(
        self,
        p1: EntityID,
        p2: EntityID,
        center: EntityID | None = None,
        axis: EntityID | None = None,
        user_visible: bool = True,
    ):
        self.p1 = p1
        self.p2 = p2
        self.center = center
        self.axis = axis

    @classmethod
    def get_type_key(cls) -> str:
        return "symmetry"

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        if len(selection.point_ids) == 3 and not selection.entity_ids:
            return True
        if len(selection.point_ids) == 2 and len(selection.entity_ids) == 1:
            entities = selection.resolve_entities(
                sketch.registry if sketch else None
            )
            return entities is not None and cls.applies_to_entities(entities)
        return False

    @classmethod
    def applies_to_entities(cls, entities: Sequence[Entity]) -> bool:
        """The operand must be a Line (the symmetry axis)."""
        return len(entities) == 1 and isinstance(entities[0], Line)

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Symmetry")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: EntityRegistry) -> str:
        """Returns a human-readable subtitle describing constrained points."""
        p1 = registry.get_point(self.p1)
        p2 = registry.get_point(self.p2)
        if p1 and p2:
            return _("From {} to {}").format(
                self._format_coord(p1.x, p1.y),
                self._format_coord(p2.x, p2.y),
            )
        return ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> SymmetryConstraint:
        return cls(
            p1=data["p1"],
            p2=data["p2"],
            center=data.get("center"),
            axis=data.get("axis"),
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
        p1 = reg.get_point(self.p1)
        p2 = reg.get_point(self.p2)
        if not (p1 and p2):
            return False
        s1 = to_screen((p1.x, p1.y))
        s2 = to_screen((p2.x, p2.y))
        mx = (s1[0] + s2[0]) / 2.0
        my = (s1[1] + s2[1]) / 2.0
        angle = math.atan2(s2[1] - s1[1], s2[0] - s1[0])
        offset = 12.0
        lx = mx - offset * math.cos(angle)
        ly = my - offset * math.sin(angle)
        rx = mx + offset * math.cos(angle)
        ry = my + offset * math.sin(angle)
        if math.hypot(sx - lx, sy - ly) < threshold:
            return True
        return math.hypot(sx - rx, sy - ry) < threshold

    def error(
        self, reg: EntityRegistry, params: ParameterContext
    ) -> list[float]:
        return super().error(reg)

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

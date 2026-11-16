from __future__ import annotations

import math
from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

from raygeo.geo.types import Point
from raygeo.sketch.constraints.vertical import (
    VerticalConstraint as _CoreVerticalConstraint,
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


class VerticalConstraint(_CoreVerticalConstraint, Constraint):
    """Enforces two points have the same X coordinate."""

    def __new__(cls, p1, p2, user_visible=True):
        return super().__new__(cls, p1, p2, user_visible)

    def __init__(self, p1: EntityID, p2: EntityID, user_visible: bool = True):
        self.p1: EntityID = p1
        self.p2: EntityID = p2

    def is_world_anchored(self) -> bool:
        return True

    @classmethod
    def get_type_key(cls) -> str:
        return "vert"

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
        return entities is not None and cls.applies_to_entities(entities)

    @classmethod
    def applies_to_entities(cls, entities: Sequence[Entity]) -> bool:
        """All operands must be Lines."""
        return bool(entities) and all(isinstance(e, Line) for e in entities)

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Vertical")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: EntityRegistry) -> str:
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

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> VerticalConstraint:
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
        p1 = reg.get_point(self.p1)
        p2 = reg.get_point(self.p2)
        if p1 and p2:
            s1 = to_screen((p1.x, p1.y))
            s2 = to_screen((p2.x, p2.y))

            t = 0.2
            mx = s1[0] + (s2[0] - s1[0]) * t
            my = s1[1] + (s2[1] - s1[1]) * t
            cx = mx + 10
            cy = my
            return math.hypot(sx - cx, sy - cy) < threshold
        return False

    def error(self, reg: EntityRegistry, params: ParameterContext) -> float:
        return super().error(reg)[0]

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

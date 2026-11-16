from __future__ import annotations

from gettext import gettext as _
from typing import TYPE_CHECKING, Any

from raygeo.geo.types import Point
from raygeo.sketch.constraints.collinear import (
    CollinearConstraint as _CoreCollinearConstraint,
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


class CollinearConstraint(_CoreCollinearConstraint, Constraint):
    """Enforces that three points (p1, p2, p3) lie on the same line."""

    def __new__(cls, p1, p2, p3, user_visible=True):
        return super().__new__(cls, p1, p2, p3, user_visible)

    def __init__(
        self,
        p1: EntityID,
        p2: EntityID,
        p3: EntityID,
        user_visible: bool = True,
    ):
        self.p1: EntityID = p1
        self.p2: EntityID = p2
        self.p3: EntityID = p3

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        return len(selection.point_ids) == 3 and not selection.entity_ids

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Collinear")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: EntityRegistry) -> str:
        """Returns a human-readable subtitle describing constrained points."""
        p1 = registry.get_point(self.p1)
        p2 = registry.get_point(self.p2)
        p3 = registry.get_point(self.p3)
        if p1 and p2 and p3:
            return _("{}, {}, {}").format(
                self._format_coord(p1.x, p1.y),
                self._format_coord(p2.x, p2.y),
                self._format_coord(p3.x, p3.y),
            )
        return ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> CollinearConstraint:
        return cls(
            p1=data["p1"],
            p2=data["p2"],
            p3=data["p3"],
            user_visible=data.get("user_visible", True),
        )

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

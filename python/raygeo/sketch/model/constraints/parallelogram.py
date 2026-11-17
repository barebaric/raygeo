from __future__ import annotations

from gettext import gettext as _
from typing import TYPE_CHECKING, Any

from raygeo.geo.types import Point
from raygeo.sketch.constraints.parallelogram import (
    ParallelogramConstraint as _CoreParallelogramConstraint,
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


class ParallelogramConstraint(_CoreParallelogramConstraint, Constraint):
    """Enforces four points form a parallelogram."""

    def __new__(cls, p_origin, p_width, p_height, p4, user_visible=False):
        obj = super().__new__(
            cls, p_origin, p_width, p_height, p4, user_visible
        )
        obj.seed_reference_attrs()
        return obj

    def __init__(
        self,
        p_origin: EntityID,
        p_width: EntityID,
        p_height: EntityID,
        p4: EntityID,
        user_visible: bool = False,
    ):
        self.p_origin: EntityID = p_origin
        self.p_width: EntityID = p_width
        self.p_height: EntityID = p_height
        self.p4: EntityID = p4

    def get_referenced_point_ids(self) -> set[EntityID]:
        return {self.p_origin, self.p_width, self.p_height, self.p4}

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        return len(selection.point_ids) == 4 and not selection.entity_ids

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Parallelogram")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: EntityRegistry) -> str:
        """Returns a human-readable subtitle describing constrained points."""
        p_origin = registry.get_point(self.p_origin)
        if p_origin:
            return _("Origin at {}").format(
                self._format_coord(p_origin.x, p_origin.y)
            )
        return ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ParallelogramConstraint:
        return cls(
            p_origin=data["p_origin"],
            p_width=data["p_width"],
            p_height=data["p_height"],
            p4=data["p4"],
            user_visible=data.get("user_visible", False),
        )

    def error(self, reg: EntityRegistry, params: ParameterContext) -> Point:
        err = super().error(reg)
        return (err[0], err[1])

    def gradient(
        self, reg: EntityRegistry, params: ParameterContext
    ) -> dict[EntityID, list[Point]]:
        return super().gradient(reg)

    def to_dict(self) -> dict[str, Any]:
        return super().to_dict()

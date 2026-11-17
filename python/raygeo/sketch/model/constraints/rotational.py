from __future__ import annotations

import math
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

from raygeo.geo.types import Point
from raygeo.sketch.constraints.rotational import (
    RotationalConstraint as _CoreRotationalConstraint,
)

from .base import Constraint

if TYPE_CHECKING:
    from raygeo.sketch.model.protocols import (
        ParameterContext,
    )
    from raygeo.sketch.types import EntityID


class RotationalConstraint(_CoreRotationalConstraint, Constraint):
    """
    Links a target point (p2) to a source point (p1) through a rotation
    of `value` radians around a center point.

    Used by array commands to keep array copies parametrically attached
    to the template member: editing any instance propagates to the whole
    array through the solver. Deleting a member deletes only its own
    constraints, so the remaining instances are never redistributed.
    """

    def __new__(
        cls, center, p1, p2, value, expression=None, user_visible=False
    ):
        obj = super().__new__(cls, center, p1, p2, 0.0, user_visible)
        obj.seed_reference_attrs()
        return obj

    def __init__(
        self,
        center: EntityID,
        p1: EntityID,
        p2: EntityID,
        value: str | float,
        expression: str | None = None,
        user_visible: bool = False,
    ):
        self.center: EntityID = center
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
        return "rotational"

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Rotational Array")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        angle_deg = math.degrees(self.value)
        return f"{self.get_type_name()} {angle_deg:.1f}°"

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> RotationalConstraint:
        return cls(
            center=data["center"],
            p1=data["p1"],
            p2=data["p2"],
            value=data["value"],
            expression=data.get("expression"),
            user_visible=data.get("user_visible", False),
        )

    def get_referenced_point_ids(self) -> set[EntityID]:
        return Constraint.get_referenced_point_ids(self)

    def get_referenced_entity_ids(self) -> set[EntityID]:
        return Constraint.get_referenced_entity_ids(self)

    def error(self, reg: Any, params: ParameterContext) -> list[float]:
        return super().error(reg)

    def gradient(
        self, reg: Any, params: ParameterContext
    ) -> dict[EntityID, list[Point]]:
        return super().gradient(reg)

    def to_dict(self) -> dict[str, Any]:
        return super().to_dict()

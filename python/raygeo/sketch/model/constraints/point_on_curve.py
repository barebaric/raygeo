from __future__ import annotations

import math
from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

from raygeo.geo.types import Point
from raygeo.sketch.constraints.point_on_curve import (
    PointOnCurveConstraint as _CorePointOnCurveConstraint,
)
from raygeo.sketch.entities import Bezier

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

_CURVE_SAMPLES = 24
_NEWTON_ITERATIONS = 12
_MIN_DISTANCE = 1e-9


class PointOnCurveConstraint(_CorePointOnCurveConstraint, Constraint):
    """Enforces a point lies on a Bezier curve."""

    def __new__(cls, point_id, shape_id, user_visible=True):
        return super().__new__(cls, point_id, shape_id, user_visible)

    def __init__(
        self, point_id: EntityID, shape_id: EntityID, user_visible: bool = True
    ):
        self.point_id: EntityID = point_id
        self.shape_id: EntityID = shape_id

    @classmethod
    def get_type_key(cls) -> str:
        return "point_on_curve"

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        if len(selection.point_ids) != 1 or len(selection.entity_ids) != 1:
            return False
        entities = selection.resolve_entities(
            sketch.registry if sketch else None
        )
        if entities is None or not cls.applies_to_entities(entities):
            return False
        return selection.point_ids[0] not in entities[0].get_endpoint_ids()

    @classmethod
    def applies_to_entities(cls, entities: Sequence[Entity]) -> bool:
        """The shape must support point-on-curve constraints."""
        return len(entities) == 1 and entities[0].supports_point_on_curve()

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Point on Curve")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: EntityRegistry) -> str:
        """Returns subtitle describing constrained entities."""
        pt = registry.get_point(self.point_id)
        if pt:
            return _("Point at {}").format(self._format_coord(pt.x, pt.y))
        return ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> PointOnCurveConstraint:
        return cls(
            point_id=data["point_id"],
            shape_id=data["shape_id"],
            user_visible=data.get("user_visible", True),
        )

    def _get_bezier(self, reg: EntityRegistry) -> Bezier | None:
        bez = reg.get_entity(self.shape_id)
        return bez if isinstance(bez, Bezier) else None

    def is_hit(
        self,
        sx: float,
        sy: float,
        reg: EntityRegistry,
        to_screen: Callable[[Point], Point],
        element: Any,
        threshold: float,
    ) -> bool:
        pt = reg.get_point(self.point_id)
        if pt:
            s_pt = to_screen((pt.x, pt.y))
            return math.hypot(sx - s_pt[0], sy - s_pt[1]) < threshold
        return False

    def get_draggable_point(self) -> EntityID:
        """Returns the point that lies on the curve."""
        return self.point_id

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

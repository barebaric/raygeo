from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING, Any

from raygeo.geo.types import Point
from raygeo.sketch.constraints.drag import (
    DragConstraint as _CoreDragConstraint,
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


class DragConstraint(_CoreDragConstraint, Constraint):
    """
    A transient constraint used only during interaction.
    It pulls a point toward a target (mouse) coordinate.
    """

    def __new__(
        cls, point_id, target_x, target_y, weight=0.1, user_visible=True
    ):
        return super().__new__(
            cls, point_id, target_x, target_y, weight, user_visible
        )

    def __init__(
        self,
        point_id: EntityID,
        target_x: float,
        target_y: float,
        weight: float = 0.1,
        user_visible: bool = True,
    ):
        self.point_id = point_id
        self.target_x = target_x
        self.target_y = target_y
        self.weight = weight

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        return False

    def error(
        self, reg: EntityRegistry, params: ParameterContext
    ) -> tuple[float, float]:
        err = super().error(reg)
        return (err[0], err[1])

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

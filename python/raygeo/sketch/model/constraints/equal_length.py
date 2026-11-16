from __future__ import annotations

import math
from collections.abc import Callable, Sequence
from gettext import gettext as _
from typing import TYPE_CHECKING, Any

import cairo
from gi.repository import PangoCairo

from raygeo.geo.types import Point
from raygeo.sketch.constraints.equal_length import (
    EqualLengthConstraint as _CoreEqualLengthConstraint,
)
from raygeo.sketch.entities import Arc, Circle, Ellipse, Line
from raygeo.sketch.model.text import create_text_layout

from .base import Constraint, ConstraintStatus

if TYPE_CHECKING:
    from raygeo.sketch.entities import Arc, Circle, Ellipse, Entity, Line
    from raygeo.sketch.model.protocols import (
        ParameterContext,
        Sketch,
        SketchSelection,
    )
    from raygeo.sketch.registry import EntityRegistry
    from raygeo.sketch.types import EntityID


class EqualLengthConstraint(_CoreEqualLengthConstraint, Constraint):
    """
    Enforces that all entities in a set have the same characteristic length.
    - Line: Length
    - Arc/Circle: Radius
    - Ellipse: Both radii (X and Y)
    """

    def __new__(cls, entity_ids, user_visible=True):
        return super().__new__(cls, entity_ids, user_visible)

    def __init__(self, entity_ids: list[EntityID], user_visible: bool = True):
        self.entity_ids = entity_ids

    @classmethod
    def get_type_key(cls) -> str:
        return "equal"

    @classmethod
    def can_apply_to(
        cls, selection: SketchSelection, sketch: Sketch | None = None
    ) -> bool:
        if selection.point_ids or len(selection.entity_ids) < 2:
            return False
        entities = selection.resolve_entities(
            sketch.registry if sketch else None
        )
        return entities is not None and cls.applies_to_entities(entities)

    @classmethod
    def applies_to_entities(cls, entities: Sequence[Entity]) -> bool:
        """All operands must define a characteristic length."""
        return len(entities) >= 2 and all(
            e.characteristic_length_pairs() for e in entities
        )

    @staticmethod
    def get_type_name() -> str:
        """Returns to human-readable name of this constraint type."""
        return _("Equal Length")

    def get_title(self) -> str:
        """Returns a human-readable title for this constraint."""
        return self.get_type_name()

    def get_subtitle(self, registry: EntityRegistry) -> str:
        """Returns subtitle describing constrained entities."""
        if len(self.entity_ids) >= 2:
            return _("{} entities").format(len(self.entity_ids))
        return ""

    def targets_segment(
        self, p1: EntityID, p2: EntityID, entity_id: EntityID | None
    ) -> bool:
        if entity_id is not None:
            return entity_id in self.entity_ids
        return False

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> EqualLengthConstraint:
        return cls(
            entity_ids=data["entity_ids"],
            user_visible=data.get("user_visible", True),
        )

    def constrains_radius(
        self, registry: EntityRegistry, entity_id: EntityID
    ) -> bool:
        return entity_id in self.entity_ids

    def get_referenced_entity_ids(self) -> set[EntityID]:
        return set(self.entity_ids)

    def _pair_dist(self, pa_idx, pb_idx, reg):
        pa = reg.get_point(pa_idx)
        pb = reg.get_point(pb_idx)
        if not (pa and pb):
            return 0.0
        return math.hypot(pb.x - pa.x, pb.y - pa.y)

    def _get_symbol_pos(
        self,
        entity,
        reg: EntityRegistry,
        to_screen: Callable[[Point], Point],
    ):
        """Calculates screen pos for an equality symbol on an entity."""
        # 1. Get anchor point (mid_x, mid_y) and normal_angle in MODEL space
        mid_x, mid_y, normal_angle = 0.0, 0.0, 0.0

        if isinstance(entity, Line):
            p1 = reg.get_point(entity.p1_idx)
            p2 = reg.get_point(entity.p2_idx)
            mid_x = (p1.x + p2.x) / 2.0
            mid_y = (p1.y + p2.y) / 2.0
            tangent_angle = math.atan2(p2.y - p1.y, p2.x - p1.x)
            normal_angle = tangent_angle - (math.pi / 2.0)
        elif isinstance(entity, (Arc, Circle, Ellipse)):
            midpoint = entity.get_midpoint(reg)
            if not midpoint:
                return None
            mid_x, mid_y = midpoint
            center = reg.get_point(entity.center_idx)
            normal_angle = math.atan2(mid_y - center.y, mid_x - center.x)

        # Estimate scale from transform
        p0 = to_screen((0, 0))
        p1 = to_screen((1, 0))
        scale = math.hypot(p1[0] - p0[0], p1[1] - p0[1])
        if scale < 1e-9:
            scale = 1.0

        offset_dist_model = 15.0 / scale
        final_x = mid_x + offset_dist_model * math.cos(normal_angle)
        final_y = mid_y + offset_dist_model * math.sin(normal_angle)
        return to_screen((final_x, final_y))

    def is_hit(
        self,
        sx: float,
        sy: float,
        reg: EntityRegistry,
        to_screen: Callable[[Point], Point],
        element: Any,
        threshold: float,
    ) -> bool:
        for entity_id in self.entity_ids:
            entity = reg.get_entity(entity_id)
            if not entity:
                continue
            pos = self._get_symbol_pos(entity, reg, to_screen)
            if pos:
                esx, esy = pos
                if math.hypot(sx - esx, sy - esy) < threshold:
                    return True
        return False

    def draw(
        self,
        ctx: cairo.Context,
        registry: EntityRegistry,
        to_screen: Callable[[Point], Point],
        is_selected: bool = False,
        is_hovered: bool = False,
        point_radius: float = 5.0,
    ) -> None:
        for entity_id in self.entity_ids:
            entity = registry.get_entity(entity_id)
            if not entity:
                continue

            pos = self._get_symbol_pos(entity, registry, to_screen)
            if not pos:
                continue

            sx, sy = pos
            ctx.save()

            if is_selected:
                ctx.set_source_rgba(0.2, 0.6, 1.0, 0.4)
                ctx.arc(sx, sy, 10, 0, 2 * math.pi)
                ctx.fill()

            if self.status == ConstraintStatus.CONFLICTING:
                ctx.set_source_rgba(1.0, 0.2, 0.2, 0.5)
                ctx.arc(sx, sy, 12, 0, 2 * math.pi)
                ctx.fill()

            self._set_color(ctx, is_hovered)
            layout = create_text_layout(ctx, "=", 16)
            ext, _ = layout.get_pixel_extents()
            ctx.move_to(
                sx - ext.width / 2 - ext.x, sy - ext.height / 2 - ext.y
            )
            PangoCairo.show_layout(ctx, layout)
            ctx.restore()
            ctx.new_path()

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

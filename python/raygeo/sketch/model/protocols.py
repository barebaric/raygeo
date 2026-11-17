"""Typing-only protocols for app-layer collaborators of the model.

These types live in the application (rayforge): the sketch document,
its parameter context, and the selection facade.  The model layer
only annotates them, so runtime behavior is duck-typed.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Protocol, Sequence, runtime_checkable

if TYPE_CHECKING:
    from raygeo.sketch.entities import Entity
    from raygeo.sketch.types import EntityID


@runtime_checkable
class ParameterContext(Protocol):
    """Read-only view over named parameters (expressions)."""

    def get(self, name: str) -> float | None: ...


@runtime_checkable
class SketchSelection(Protocol):
    """Selection facade: ids plus entity resolution."""

    point_ids: list[EntityID]
    entity_ids: list[EntityID]

    def resolve_entities(self, registry: Any) -> Sequence[Entity] | None: ...


@runtime_checkable
class Sketch(Protocol):
    """Minimal sketch facade used by ``can_apply_to`` checks."""

    registry: Any

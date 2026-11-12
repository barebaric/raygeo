"""Protocols for the sketch symbol rendering APIs.

The constraint `draw` and `is_hit` methods accept the caller's own
objects duck-typed against these protocols: the canvas the symbols
replay onto, the document the symbol layout reads through, and the
solver constraint classes.
"""

from collections.abc import Callable
from typing import Protocol, runtime_checkable

ScreenMap = Callable[[tuple[float, float]], tuple[float, float]]


@runtime_checkable
class Canvas(Protocol):
    """Subset of the cairo context API the symbols replay onto."""

    def save(self) -> None: ...

    def restore(self) -> None: ...

    def set_line_width(self, width: float) -> None: ...

    def set_dash(self, on: float, off: float) -> None: ...

    def set_line_cap(self, cap: str) -> None: ...

    def set_line_join(self, join: str) -> None: ...

    def set_source_rgb(self, r: float, g: float, b: float) -> None: ...

    def set_source_rgba(
        self, r: float, g: float, b: float, a: float
    ) -> None: ...

    def move_to(self, x: float, y: float) -> None: ...

    def line_to(self, x: float, y: float) -> None: ...

    def arc(
        self,
        x: float,
        y: float,
        radius: float,
        start_angle: float,
        end_angle: float,
    ) -> None: ...

    def arc_negative(
        self,
        x: float,
        y: float,
        radius: float,
        start_angle: float,
        end_angle: float,
    ) -> None: ...

    def rectangle(
        self, x: float, y: float, width: float, height: float
    ) -> None: ...

    def fill(self) -> None: ...

    def stroke(self) -> None: ...

    def stroke_preserve(self) -> None: ...

    def new_sub_path(self) -> None: ...

    def new_path(self) -> None: ...

    def get_line_width(self) -> float: ...

    def translate(self, x: float, y: float) -> None: ...

    def rotate(self, angle: float) -> None: ...


@runtime_checkable
class DrawContext(Protocol):
    """Read view over the caller's sketch document, addressed by
    point id."""

    def get_point(self, id: int) -> tuple[float, float] | None: ...

    def point_is_fixed(self, id: int) -> bool: ...

    def entity_ids(self) -> list[int]: ...

    def entity_type(self, id: int) -> str | None: ...

    def entity_point_ids(self, id: int) -> list[int] | None: ...

    def point_is_in_text_box_frame(self, id: int) -> bool: ...


@runtime_checkable
class SolverConstraint(Protocol):
    """Runtime surface of the solver constraint classes."""

    status: int

    def draw(
        self,
        ctx: Canvas,
        registry: DrawContext,
        to_screen: ScreenMap,
        is_selected: bool = False,
        is_hovered: bool = False,
        point_radius: float = 5.0,
    ) -> None: ...

    def error(self, registry: DrawContext) -> list[float]: ...

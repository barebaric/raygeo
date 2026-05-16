"""PyO3-based geometry engine for RayForge.

This module provides core geometric primitives and algorithms including
path construction, polygon operations, shape queries, and curve fitting.
It is implemented as a native Rust extension for performance.

Type Aliases:
    Point: A 2D point as ``(x, y)``.
    Point3D: A 3D point as ``(x, y, z)``.
    Rect: An axis-aligned bounding box as ``(x_min, y_min, x_max, y_max)``.
    Polygon: A 2D polygon as a list of :data:`Point` vertices.
    Polygon3D: A 3D polygon as a list of :data:`Point3D` vertices.
    IntPoint: An integer 2D point as ``(x, y)``.
    IntPolygon: An integer polygon as a list of :data:`IntPoint`.
    Edge: A line segment as a pair of :data:`Point` endpoints.
    CubicBezier: A cubic Bezier curve as four :data:`Point` control points.
    Point2DOr3D: A point that is either :data:`Point` or :data:`Point3D`.
"""

from typing import Tuple, List, Union, Optional, Sequence, TypeAlias
from collections import namedtuple

from numpy.typing import NDArray
import numpy as np

Point: TypeAlias = Tuple[float, float]
Point3D: TypeAlias = Tuple[float, float, float]
Rect: TypeAlias = Tuple[float, float, float, float]
Polygon: TypeAlias = List[Tuple[float, float]]
Polygon3D: TypeAlias = List[Tuple[float, float, float]]
IntPoint: TypeAlias = Tuple[int, int]
IntPolygon: TypeAlias = List[Tuple[int, int]]
Edge: TypeAlias = Tuple[Tuple[float, float], Tuple[float, float]]
CubicBezier: TypeAlias = Tuple[
    Tuple[float, float],
    Tuple[float, float],
    Tuple[float, float],
    Tuple[float, float],
]
Point2DOr3D: TypeAlias = Union[Tuple[float, float], Tuple[float, float, float]]

_PolygonsInput: TypeAlias = Union[
    List[Polygon],
    List[NDArray[np.float64]],
]

_Point2DOr3D: TypeAlias = Tuple[float, ...]
_CommandRow = Tuple[float, float, float, float, float, float, float, float]
_ArrayLike: TypeAlias = Union[List[List[float]], NDArray[np.float64]]


class Rect3D(
    namedtuple(
        "Rect3D", ["x_min", "x_max", "y_min", "y_max", "z_min", "z_max"]
    )
):
    """A 3D axis-aligned bounding box with separate min/max for each axis."""


CMD_TYPE_MOVE: int
CMD_TYPE_LINE: int
CMD_TYPE_ARC: int
CMD_TYPE_BEZIER: int
COL_TYPE: int
COL_X: int
COL_Y: int
COL_Z: int
COL_I: int
COL_J: int
COL_CW: int
COL_C1X: int
COL_C1Y: int
COL_C2X: int
COL_C2Y: int
GEO_ARRAY_COLS: int
CLIPPER_SCALE: int


class constants:
    CMD_TYPE_MOVE: int
    CMD_TYPE_LINE: int
    CMD_TYPE_ARC: int
    CMD_TYPE_BEZIER: int
    COL_TYPE: int
    COL_X: int
    COL_Y: int
    COL_Z: int
    COL_I: int
    COL_J: int
    COL_CW: int
    COL_C1X: int
    COL_C1Y: int
    COL_C2X: int
    COL_C2Y: int
    GEO_ARRAY_COLS: int


class PyCommand:
    """Typed view over a single geometry command row."""

    end: Point3D

    class Move:
        end: Point3D

    class Line:
        end: Point3D

    class Arc:
        end: Point3D
        center_offset: Point
        clockwise: bool

    class Bezier:
        end: Point3D
        control1: Point
        control2: Point


class Geometry:
    COL_TYPE: int
    COL_X: int
    COL_Y: int
    COL_Z: int
    COL_I: int
    COL_J: int
    COL_CW: int
    COL_C1X: int
    COL_C1Y: int
    COL_C2X: int
    COL_C2Y: int
    CMD_TYPE_MOVE: float
    CMD_TYPE_LINE: float
    CMD_TYPE_ARC: float
    CMD_TYPE_BEZIER: float

    def __init__(self) -> None: ...
    def move_to(self, x: float, y: float, z: float = ...) -> None: ...
    def line_to(self, x: float, y: float, z: float = ...) -> None: ...
    def close_path(self) -> None: ...
    def arc_to(
        self,
        x: float,
        y: float,
        i: float = ...,
        j: float = ...,
        clockwise: bool = ...,
        z: float = ...,
    ) -> None: ...
    def bezier_to(
        self,
        x: float,
        y: float,
        c1x: float,
        c1y: float,
        c2x: float,
        c2y: float,
        z: float = ...,
    ) -> None: ...
    def arc_to_as_bezier(
        self,
        x: float,
        y: float,
        i: float,
        j: float,
        clockwise: bool = ...,
        z: float = ...,
    ) -> None: ...
    def sync_to_data(self) -> None: ...
    def _sync_to_numpy(self) -> None: ...
    def __len__(self) -> int: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...
    def __add__(self, other: "Geometry") -> "Geometry": ...

    @property
    def data(self) -> Optional[NDArray[np.float64]]: ...
    @data.setter
    def data(self, value: Optional[NDArray[np.float64]]) -> None: ...
    @property
    def last_move_to(self) -> Point3D: ...
    @last_move_to.setter
    def last_move_to(self, value: Point3D) -> None: ...
    @property
    def uniform_scalable(self) -> bool: ...
    @uniform_scalable.setter
    def uniform_scalable(self, value: bool) -> None: ...
    @property
    def _pending_data(self) -> List[List[float]]: ...
    def _get_last_point(self) -> Point3D: ...

    def is_empty(self) -> bool: ...
    def rect(self) -> Tuple[float, float, float, float]: ...
    def distance(self) -> float: ...
    def area(self) -> float: ...
    def is_closed(self, tolerance: float = ...) -> bool: ...
    def segments(self) -> List[List[Point3D]]: ...
    def get_command_at(self, index: int) -> Optional[_CommandRow]: ...
    def iter_commands(self) -> List[_CommandRow]: ...
    def iter_typed_commands(self) -> List[PyCommand]: ...
    def get_typed_command_at(self, index: int) -> Optional[PyCommand]: ...
    def find_closest_point(
        self, x: float, y: float,
    ) -> Optional[Tuple[int, float, Point]]: ...
    def get_point_and_tangent_at(
        self, segment_index: int, t: float,
    ) -> Optional[Tuple[Point, Point]]: ...
    def get_outward_normal_at(
        self, segment_index: int, t: float,
    ) -> Optional[Point]: ...
    def has_self_intersections(self, fail_on_t_junction: bool = ...) -> bool: ...
    def intersects_with(self, other: Geometry) -> bool: ...
    def encloses(self, other: Geometry) -> bool: ...

    def clear(self) -> None: ...
    def copy(self) -> "Geometry": ...
    def transform(
        self, matrix: Union[List[List[float]], NDArray[np.float64]]
    ) -> "Geometry": ...
    def extend(self, other: "Geometry") -> None: ...
    def simplify(self, tolerance: float) -> "Geometry": ...
    def linearize(self, tolerance: float) -> "Geometry": ...
    def fit_curves(
        self,
        tolerance: float,
        beziers: bool,
        arcs: bool,
        on_progress: Optional[object] = ...,
    ) -> "Geometry": ...
    def fit_arcs(self, tolerance: float) -> "Geometry": ...
    def upgrade_to_scalable(self) -> "Geometry": ...
    def close_gaps(self, tolerance: float = ...) -> "Geometry": ...
    def cleanup(self, tolerance: float) -> "Geometry": ...
    def append_data(self, rows: Optional[NDArray[np.float64]] = ...) -> None: ...
    def flip_x(self) -> "Geometry": ...
    def flip_y(self) -> "Geometry": ...
    def grow(self, amount: float) -> "Geometry": ...
    def remove_inner_edges(self) -> "Geometry": ...
    def split_inner_and_outer_contours(
        self,
    ) -> Tuple[List["Geometry"], List["Geometry"]]: ...
    def map_to_frame(
        self,
        origin: Point,
        p_width: Tuple[float, float],
        p_height: Tuple[float, float],
        anchor_y: Optional[float] = ...,
        stable_src_height: Optional[float] = ...,
        anchor_x: Optional[float] = ...,
        stable_src_width: Optional[float] = ...,
    ) -> "Geometry": ...
    def split_into_contours(self) -> List["Geometry"]: ...
    def split_into_components(self) -> List["Geometry"]: ...
    def to_polygons(self, tolerance: float = ...) -> List[Polygon]: ...

    def dump(self) -> dict: ...
    def to_dict(self) -> dict: ...
    @classmethod
    def load(cls, data: dict) -> "Geometry": ...
    @classmethod
    def from_dict(cls, data: dict) -> "Geometry": ...
    @classmethod
    def from_points(
        cls, points: Sequence[_Point2DOr3D], close: bool = ...,
    ) -> "Geometry": ...


def clip_line_segment_with_polygons(
    p1: Point3D,
    p2: Point3D,
    regions: _PolygonsInput,
) -> List[Tuple[Point3D, Point3D]]: ...


def is_arc_inside_polygons(
    arc_start: Point,
    arc_end: Point,
    arc_center: Point,
    clockwise: bool,
    polygons: _PolygonsInput,
) -> bool: ...


def is_bezier_inside_polygons(
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
    polygons: _PolygonsInput,
) -> bool: ...


def fit_points_with_primitives(
    points: List[Point3D],
    tolerance: float,
) -> List[List[float]]: ...


def to_clipper(
    polygon: Union[Polygon, NDArray[np.float64]],
    scale: Optional[int] = ...,
) -> List[Tuple[int, int]]: ...


def from_clipper(
    polygon: List[Tuple[int, int]],
    scale: Optional[int] = ...,
) -> Polygon: ...

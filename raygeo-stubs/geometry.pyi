"""Re-exports for the ``raygeo.geometry`` submodule.

The ``Geometry`` class and related types are defined in :mod:`raygeo`.
This module exists so that ``from raygeo.geometry import Geometry``
resolves correctly for type-checking.
"""

from raygeo import (
    Geometry,
    Point,
    Point3D,
    Polygon,
    _Point2DOr3D,
    _CommandRow,
)

__all__ = [
    "Geometry",
    "Point",
    "Point3D",
    "Polygon",
    "_Point2DOr3D",
    "_CommandRow",
]

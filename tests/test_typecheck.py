from raygeo.formats.svg import svg_string_to_geometry
from raygeo.geo import Geometry
from raygeo.ops import Ops

g: Geometry
o: Ops


def parse(svg: str) -> Geometry:
    return svg_string_to_geometry(svg)

"""Generate visualisations of spiral entry motion assembly."""

from raygeo import Part
from raygeo.ops.assembly.spiral import generate_spiral
from tools.plot import plot_ops


def generate_spiral_example():
    """Spiral to ops."""
    result = generate_spiral(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        z=-5.0,
        start_radius=3.0,
        end_radius=25.0,
        revolutions=3.5,
        direction="CW",
        angular_step=0.1,
    )

    fig = plot_ops(result.ops)
    fig.suptitle("Spiral Entry Path")
    return fig


__docs_target__ = ["raygeo.ops.assembly.spiral.md"]
__images__ = [
    {
        "heading": "generate_spiral",
        "caption": "Flat Archimedean spiral with smoothing circular pass",
        "function": generate_spiral_example,
    },
]

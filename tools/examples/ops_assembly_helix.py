"""Generate visualisations of helix entry motion assembly."""

import matplotlib.pyplot as plt

from raygeo.ops.assembly.helix import generate_helix
from raygeo.ops.part import Part
from tools.plot import plot_ops_3d


def generate_helix_example():
    """Helix to ops."""
    result = generate_helix(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        start_radius=8.0,
        z_start=2.0,
        z_end=-10.0,
        pitch=3.0,
        direction="CW",
        angular_step=0.1,
    )

    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")

    plot_ops_3d(ax, result.ops)

    ax.plot([0, 0], [0, 0], [2.0, -10.0], "k--", alpha=0.3, linewidth=1)
    ax.set_title("Helical Entry Path")

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.helix.md"]
__images__ = [
    {
        "heading": "generate_helix",
        "caption": "Helical entry path from safe Z to target depth",
        "function": generate_helix_example,
    },
]

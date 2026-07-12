"""Generate visualisations of toroid entry motion assembly."""

import matplotlib.pyplot as plt

from raygeo.ops.assembly.toroid import generate_toroid
from raygeo.ops.part import Part
from tools.plot import plot_ops_2d


def generate_toroid_example():
    """Toroid to ops."""
    carrier = [(0.0, 0.0), (80.0, 0.0)]
    result = generate_toroid(
        Part.from_polygons([]),
        carrier=carrier,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
    )

    fig, ax = plt.subplots(figsize=(10, 6))

    # Draw carrier
    carrier_x = [p[0] for p in carrier]
    carrier_y = [p[1] for p in carrier]
    ax.plot(
        carrier_x, carrier_y, "k--", alpha=0.3, linewidth=2, label="Carrier"
    )

    plot_ops_2d(ax, result.ops)

    ax.set_title("Toroidal (Trochoidal) Path")
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.toroid.md"]
__images__ = [
    {
        "heading": "generate_toroid",
        "caption": "Trochoidal slot path along a carrier polyline",
        "function": generate_toroid_example,
    },
]

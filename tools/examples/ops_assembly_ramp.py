"""Generate visualisations of ramp entry motion assembly."""

import matplotlib.pyplot as plt

from raygeo.ops.assembly.ramp import generate_ramp
from tools.plot import plot_ops_3d


def generate_ramp_example():
    """Ramp to ops."""
    result = generate_ramp(
        start=(0.0, 0.0),
        end=(120.0, 0.0),
        z_start=2.0,
        z_end=-8.0,
        style="zigzag",
        lateral_amplitude=4.0,
    )

    fig = plt.figure(figsize=(10, 6))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, result.ops)
    ax.set_title("ZigZag Ramp Entry Path")
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.ramp.md"]
__images__ = [
    {
        "heading": "generate_ramp",
        "caption": "ZigZag ramp entry path from safe Z to target depth",
        "function": generate_ramp_example,
    },
]

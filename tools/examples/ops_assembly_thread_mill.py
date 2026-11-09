"""Generate visualisations of thread-mill cycle assembly."""

import matplotlib.pyplot as plt

from raygeo.ops.assembly.thread_mill import generate_thread_mill
from raygeo.ops.part import Part
from raygeo.ops.state import State
from tools.plot import plot_ops_3d


def generate_thread_mill_cycle():
    """generate_thread_mill — M6x1 internal thread via helical
    interpolation with tangential lead-in/out arcs."""
    part = Part.from_polygons([])
    state = State(feed_rate=800, plunge_feed=200, spindle_rpm=3000)
    result = generate_thread_mill(
        part,
        center=(0.0, 0.0),
        nominal_diameter=6.0,
        pitch=1.0,
        tool_diameter=4.0,
        z_top=0.0,
        z_bottom=-12.0,
        handedness="RH",
        state=state,
    )
    # Linearize so the helical arcs render as the curves they are.
    result.ops.linearize_all()

    fig = plt.figure(figsize=(9, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, result.ops)
    ax.set_title(
        "generate_thread_mill — M6x1: plunge in the pilot bore, lead-in "
        "arc, one pitch per revolution, lead-out"
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.thread_mill.md"]

__images__ = [
    {
        "heading": "generate_thread_mill",
        "caption": (
            "Thread-milling cycle: tangential lead-in, helical "
            "interpolation at one pitch per revolution."
        ),
        "function": generate_thread_mill_cycle,
    },
]

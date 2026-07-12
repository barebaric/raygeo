"""Visualisation for ops/assembly/slot — back-and-forth slotting."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.lines import Line2D

from raygeo.ops.assembly.slot import generate_slot
from raygeo.ops.feature.slot_path import find_slot_path
from raygeo.ops.part import Part
from tools.plot import plot_ops_3d


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def generate_slot_3d():
    """3D back-and-forth slot path through a 40×7 mm slot.

    The carrier is derived from `find_slot_path` (Step 9a) on the slot
    polygon and the bottom edge as the entry edge. `generate_slot` then
    emits a forward pass (entry side → far side) immediately followed by a
    backward pass (far side → entry side) at constant ``target_z``.

    Colour encodes cumulative cutting arc-length from start (blue) to end
    (red); the trochoid-free linear path is visible as a single forward
    stroke followed by a single backward stroke.
    """
    slot_polygon = _rect(0, 0, 40, 7)
    entry_edges = [0]
    entry_point = (0, 0)
    tool_radius = 3.0
    target_z = -3.0

    carrier = find_slot_path(
        slot_polygon=slot_polygon,
        entry_edges=entry_edges,
        entry_point=entry_point,
        tool_radius=tool_radius,
    )
    assert carrier is not None, "expected a carrier"

    result = generate_slot(
        Part.from_polygons([]),
        carrier=carrier,
        tool_radius=tool_radius,
        target_z=target_z,
    )

    fig = plt.figure(figsize=(11, 7))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, result.ops)

    # Slot polygon outline at target_z.
    sx = [p[0] for p in slot_polygon] + [slot_polygon[0][0]]
    sy = [p[1] for p in slot_polygon] + [slot_polygon[0][1]]
    sz = [target_z] * len(sx)
    ax.plot(sx, sy, sz, "k-", linewidth=1.5, alpha=0.5)

    # Carrier at target_z.
    cx = [p[0] for p in carrier]
    cy = [p[1] for p in carrier]
    cz = [target_z] * len(carrier)
    ax.plot(cx, cy, cz, "k--", linewidth=1.2, alpha=0.6)

    # Tool-disk envelope at start and end of carrier.
    theta = np.linspace(0, 2 * np.pi, 24)
    for p in carrier:
        ex = p[0] + tool_radius * np.cos(theta)
        ey = p[1] + tool_radius * np.sin(theta)
        ez = [target_z] * len(theta)
        ax.plot(ex, ey, ez, color="navy", linewidth=0.8, alpha=0.5)

    ax.set_title(
        "generate_slot — forward + backward linear pass at constant target_z",
        fontsize=11,
    )

    legend_items = [
        Line2D(
            [0],
            [0],
            color="k",
            linestyle="-",
            linewidth=1.5,
            label="slot polygon",
        ),
        Line2D(
            [0], [0], color="k", linestyle="--", linewidth=1.2, label="carrier"
        ),
        Line2D(
            [0], [0], color="navy", linewidth=0.8, label="tool disk envelope"
        ),
    ]
    ax.legend(handles=legend_items, loc="upper right", fontsize=9)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.slot.md"]
__images__ = [
    {
        "heading": "generate_slot",
        "caption": (
            "3D forward+backward slot path through a rectangular slot at"
            " constant depth, no trochoid."
        ),
        "function": generate_slot_3d,
    },
]


if __name__ == "__main__":
    fig = generate_slot_3d()
    fig.savefig("/tmp/ops_assembly_slot.png", dpi=150, bbox_inches="tight")
    print("Saved /tmp/ops_assembly_slot.png")

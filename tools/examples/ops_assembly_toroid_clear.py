"""Generate visualisations of ramp-down toroidal clear assembly."""

import math

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

from raygeo.ops.assembly.toroid import generate_toroidal_clear
from raygeo.ops.part import Part
from tools.plot import plot_ops_3d


def generate_toroidal_clear_3d():
    """3D ramp-down toroidal clear over a carrier that is too short.

    The carrier is 20 mm long, but with ``start.z = 0``, ``target_z = -7``
    and ``max_ramp_angle_deg ≈ 6.65`` (so ``L_min = 60``), the tool must
    zig-zag back-and-forth three times while descending, then make one
    final full forward pass at constant ``target_z``. The rainbow colour
    encodes cumulative arc-length from start (blue) to end (red).
    """
    carrier = [(0.0, 0.0), (20.0, 0.0)]
    start_z = 12.0
    target_z = -5.0
    delta_z = start_z - target_z
    l_min = 60.0
    angle = math.degrees(math.atan(delta_z / l_min))

    result = generate_toroidal_clear(
        Part.from_polygons([]),
        carrier=carrier,
        start=(carrier[0][0], carrier[0][1], start_z),
        target_z=target_z,
        tool_radius=3.0,
        step_over=2.0,
        max_ramp_angle_deg=angle,
    )

    fig = plt.figure(figsize=(11, 7))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(
        ax,
        result.ops,
        mark_cut_start=False,
        mark_start=False,
        mark_end=False,
    )

    # Carrier at target_z.
    cx = [p[0] for p in carrier] + [carrier[0][0]]
    cy = [p[1] for p in carrier] + [carrier[0][1]]
    cz = [target_z] * len(cx)
    ax.plot(cx, cy, cz, "k--", linewidth=1.5, alpha=0.6)

    # Start height reference.
    xmin = min(p[0] for p in carrier)
    xmax = max(p[0] for p in carrier)
    ax.plot(
        [xmin, xmax],
        [carrier[0][1], carrier[0][1]],
        [start_z, start_z],
        color="forestgreen",
        linestyle=":",
        linewidth=1.0,
        alpha=0.5,
    )

    ax.set_title(
        "generate_toroidal_clear — zig-zag ramp-down + final flat pass",
    )

    legend_items = [
        Line2D(
            [0],
            [0],
            color=plt.cm.turbo(0.0),
            linewidth=1.5,
            label="start (blue)",
        ),
        Line2D(
            [0],
            [0],
            color=plt.cm.turbo(1.0),
            linewidth=1.5,
            label="end (red)",
        ),
        Line2D(
            [0], [0], color="k", linestyle="--", label="carrier @ target_z"
        ),
        Line2D(
            [0],
            [0],
            color="forestgreen",
            linestyle=":",
            label="start height",
        ),
        Line2D([0], [0], color="dimgray", linestyle=":", label="travel move"),
    ]
    ax.legend(handles=legend_items, loc="upper right", fontsize=9)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.toroid.md"]
__images__ = [
    {
        "heading": "generate_toroidal_clear",
        "caption": (
            "3D ramp-down toroidal clear zig-zagging along a short carrier"
            " with full ramp descent."
        ),
        "function": generate_toroidal_clear_3d,
    },
]


if __name__ == "__main__":
    fig = generate_toroidal_clear_3d()
    fig.savefig(
        "/tmp/ops_assembly_toroid_clear.png", dpi=150, bbox_inches="tight"
    )
    print("Saved /tmp/ops_assembly_toroid_clear.png")

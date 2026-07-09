"""Visualisation for ops/assembly/slot — back-and-forth slotting."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.lines import Line2D
from mpl_toolkits.mplot3d.art3d import Line3DCollection

from raygeo.ops.assembly.slot import generate_slot
from raygeo.ops.feature import slot_path as _sp

find_slot_path = _sp.find_slot_path


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _all_moving_pts(ops):
    """Extract ``(x, y, z, is_travel)`` for every moving command."""
    pts = []
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            ep = ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return pts


def _plot_3d_slot_path(
    ops, ax, title, slot_polygon, carrier, tool_radius, target_z
):
    """3D plot of the forward+backward slot path coloured by arc-length.

    Travel moves are dotted gray; cutting moves are a rainbow Line3DCollection.
    The slot polygon outline and the carrier are drawn at ``target_z`` as
    references.
    """
    pts_list = _all_moving_pts(ops)
    if not pts_list:
        return

    # Split into cutting segments (travel breaks continuity).
    segments = []
    cur = []
    for p in pts_list:
        x, y, z, is_travel = p
        if is_travel:
            if len(cur) > 1:
                segments.append(cur)
            cur = []
        else:
            cur.append((x, y, z))
    if len(cur) > 1:
        segments.append(cur)

    segs_3d = []
    cum_dists = []
    cum = 0.0
    prev = None
    for seg in segments:
        for p in seg:
            if prev is not None:
                segs_3d.append([prev, p])
                cum += math.sqrt(
                    (p[0] - prev[0]) ** 2
                    + (p[1] - prev[1]) ** 2
                    + (p[2] - prev[2]) ** 2
                )
                cum_dists.append(cum)
            prev = p
        prev = None
    total = cum if cum > 0 else 1.0
    if segs_3d:
        lc = Line3DCollection(
            segs_3d,
            colors=plt.cm.turbo([d / total for d in cum_dists]),
            linewidth=1.5,
            alpha=1.0,
        )
        ax.add_collection3d(lc)

    # Travel moves as dotted gray.
    prev = None
    for p in pts_list:
        x, y, z, is_travel = p
        if is_travel and prev is not None:
            ax.plot(
                [prev[0], x],
                [prev[1], y],
                [prev[2], z],
                linestyle=":",
                linewidth=1.0,
                color="dimgray",
                alpha=0.7,
            )
        prev = (x, y, z)

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

    xl, xr = ax.get_xlim()
    yl, yr = ax.get_ylim()
    zl, zr = ax.get_zlim()
    half_xy = max(xr - xl, yr - yl) * 0.5
    xmid = (xl + xr) * 0.5
    ymid = (yl + yr) * 0.5
    ax.set_xlim(xmid - half_xy, xmid + half_xy)
    ax.set_ylim(ymid - half_xy, ymid + half_xy)
    ax.set_zlim(zl - 0.5, zr + 0.5)

    ax.set_title(title, fontsize=11)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_zlabel("Z (mm)")
    ax.view_init(elev=25, azim=-55)


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
        carrier=carrier,
        tool_radius=tool_radius,
        target_z=target_z,
    )

    fig = plt.figure(figsize=(11, 7))
    ax = fig.add_subplot(111, projection="3d")
    _plot_3d_slot_path(
        result.ops,
        ax,
        "generate_slot — forward + backward linear pass at constant target_z",
        slot_polygon,
        carrier,
        tool_radius,
        target_z,
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
            [0], [0], color=plt.cm.turbo(1.0), linewidth=1.5, label="end (red)"
        ),
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
        Line2D([0], [0], color="dimgray", linestyle=":", label="travel move"),
    ]
    ax.legend(handles=legend_items, loc="upper right", fontsize=9)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.slot.md"]
__images__ = [
    {
        "heading": "generate_slot",
        "caption": (
            "3D forward+backward slot path through a 40×7 mm slot at"
            " constant target_z=-3, no trochoid."
        ),
        "function": generate_slot_3d,
    },
]


if __name__ == "__main__":
    fig = generate_slot_3d()
    fig.savefig("/tmp/ops_assembly_slot.png", dpi=150, bbox_inches="tight")
    print("Saved /tmp/ops_assembly_slot.png")

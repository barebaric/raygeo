"""Generate visualisations of ramp-down toroidal clear assembly."""

import math

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from mpl_toolkits.mplot3d.art3d import Line3DCollection

from raygeo.ops.assembly.toroid import generate_toroidal_clear


def _all_moving_pts(ops):
    """Extract ``(x, y, z, is_travel)`` for every moving command."""
    pts = []
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            ep = ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return pts


def _plot_3d_toolpath(ops, ax, title, carrier, target_z, start_z):
    """Plot a 3D toolpath coloured by arc-length.

    Travel moves are dotted gray; cutting moves are a rainbow Line3DCollection.
    The ``carrier`` is drawn as a black dashed line at ``target_z`` so the
    intended slot axis is visible. ``start_z`` is drawn as a dim horizontal
    reference line so the entry height is obvious.
    """
    pts_list = _all_moving_pts(ops)
    if not pts_list:
        return

    # Split into cutting segments (travel moves break continuity).
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
        lc3d = Line3DCollection(
            segs_3d,
            colors=plt.cm.turbo([d / total for d in cum_dists]),
            linewidth=0.8,
            alpha=1.0,
        )
        ax.add_collection3d(lc3d)

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

    # Carrier (slot axis) at target_z.
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

    ax.set_title(title)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_zlabel("Z (mm)")
    ax.view_init(elev=25, azim=-55)


def generate_toroidal_clear_3d():
    """3D ramp-down toroidal clear over a carrier that is too short.

    The carrier is 20 mm long, but with ``start.z = 2``, ``target_z = -4``
    and ``max_ramp_angle_deg ≈ 5.71`` (so ``L_min = 60``), the tool must
    zig-zag back-and-forth three times while descending, then make one
    final full forward pass at constant ``target_z``. The rainbow colour
    encodes cumulative arc-length from start (blue) to end (red).
    """
    carrier = [(0.0, 0.0), (20.0, 0.0)]
    start_z = 2.0
    target_z = -4.0
    delta_z = start_z - target_z
    l_min = 60.0
    angle = math.degrees(math.atan(delta_z / l_min))

    result = generate_toroidal_clear(
        carrier=carrier,
        start=(carrier[0][0], carrier[0][1], start_z),
        target_z=target_z,
        tool_radius=3.0,
        step_over=2.0,
        max_ramp_angle_deg=angle,
    )

    fig = plt.figure(figsize=(11, 7))
    ax = fig.add_subplot(111, projection="3d")
    _plot_3d_toolpath(
        result.ops,
        ax,
        "generate_toroidal_clear — zig-zag ramp-down + final flat pass",
        carrier,
        target_z,
        start_z,
    )

    # Equalise aspect along XY so the carrier scale is honest.
    xl, xr = ax.get_xlim()
    yl, yr = ax.get_ylim()
    zl, zr = ax.get_zlim()
    half_xy = max(xr - xl, yr - yl) * 0.5
    xmid = (xl + xr) * 0.5
    ymid = (yl + yr) * 0.5
    ax.set_xlim(xmid - half_xy, xmid + half_xy)
    ax.set_ylim(ymid - half_xy, ymid + half_xy)
    ax.set_zlim(zl - 1, zr + 1)

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
            "3D ramp-down toroidal clear along a 20 mm carrier. With"
            " start.z=2, target_z=-4 and max_ramp_angle_deg ≈ 5.71°"
            " (L_min = 60 mm), the tool zig-zags back-and-forth three"
            " times along the carrier while descending, then makes one"
            " final full forward pass at constant target_z=-4. Colour"
            " encodes cumulative cutting-arc-length from start (blue) to"
            " end (red); the dashed black line is the carrier at target_z."
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

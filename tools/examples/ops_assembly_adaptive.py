"""Visualisation for ops/assembly/adaptive — adaptive clearing."""

import math

import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection
from matplotlib.colors import Normalize

from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.cut.cleared_area import ClearedArea


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _seed_circle(cx, cy, r, n=64):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _ops_to_segments(ops):
    """Split Ops into list of (points, is_travel) segments."""
    segs = []
    cur_pts = []
    cur_travel = False
    for i in range(ops.len()):
        if not (ops.is_cutting(i) or ops.is_travel(i)):
            continue
        is_travel = ops.is_travel(i)
        if cur_pts and is_travel != cur_travel:
            segs.append((cur_pts, cur_travel))
            cur_pts = []
        cur_travel = is_travel
        ep = ops.endpoint(i)
        cur_pts.append((ep[0], ep[1]))
    if cur_pts:
        segs.append((cur_pts, cur_travel))
    return segs


def generate_adaptive_clearing_demo():
    """Toolpath demo with seed, cuts, and travel."""
    boundary = _rect(0, 0, 200, 200)
    ca = ClearedArea(boundary=boundary, initial=[_seed_circle(0, 0, 20)])
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=5.0,
        advance=3.0,
        cut_z=-5.0,
        safe_z=2.0,
        step_length=1.0,
        max_deflection_deg=15.0,
        wall_margin=1.0,
        area_tolerance=50.0,
    )

    fig, ax = plt.subplots(figsize=(10, 10))

    # Pocket boundary
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.4, label="Pocket boundary")

    # Seed circle (entry clearing)
    seed = _seed_circle(0, 0, 20)
    sx = [p[0] for p in seed] + [seed[0][0]]
    sy = [p[1] for p in seed] + [seed[0][1]]
    ax.fill(sx, sy, color="white", alpha=1.0)
    ax.plot(
        sx,
        sy,
        color="#aaaaaa",
        linewidth=1.2,
        alpha=1.0,
        linestyle="--",
        label="Seed clearing",
    )

    # Split into cut and travel segments
    segs = _ops_to_segments(ops)

    # Collect cut segments in order for gradient colouring
    cut_segs = [
        (pts, i) for i, (pts, is_travel) in enumerate(segs) if not is_travel
    ]

    cmap = plt.cm.turbo
    norm = Normalize(vmin=0, vmax=1)

    # Build per-edge colours based on cumulative distance along the whole
    # cut path so the full spectrum is always used, even for short paths.
    segs_list = []
    cum_dists = []
    cum = 0.0
    prev = None
    for pts, _ in cut_segs:
        for p in pts:
            if prev is not None:
                segs_list.append([prev, p])
                cum += math.hypot(p[0] - prev[0], p[1] - prev[1])
                cum_dists.append(cum)
            prev = p
    total = cum if cum > 0 else 1.0

    lc = LineCollection(
        segs_list,
        colors=cmap([d / total for d in cum_dists]),
        linewidth=0.6,
        alpha=1.0,
    )
    ax.add_collection(lc)

    # Colourbar legend for cut progress
    sm = plt.cm.ScalarMappable(cmap=cmap, norm=norm)
    sm.set_array([])
    cbar = fig.colorbar(sm, ax=ax, orientation="vertical", pad=0.02, aspect=30)
    cbar.set_label("Clearing cut progress", fontsize=8)

    # Pass 2: draw all travel segments on top
    for pts, is_travel in segs:
        if is_travel:
            xs = [p[0] for p in pts]
            ys = [p[1] for p in pts]
            ax.plot(
                xs,
                ys,
                color="#888888",
                linestyle=":",
                linewidth=1.2,
                alpha=0.8,
                dashes=(1, 2),
            )

    # Mark start position of each cutting segment
    start_xs = [pts[0][0] for pts, is_travel in segs if not is_travel and pts]
    start_ys = [pts[0][1] for pts, is_travel in segs if not is_travel and pts]
    ax.scatter(
        start_xs,
        start_ys,
        marker="o",
        color="#e31a1c",
        s=12,
        zorder=5,
        label="Cut segment start",
    )

    # Single legend entry for travel (cuts shown via colourbar)
    ax.plot(
        [],
        [],
        color="#888888",
        linestyle=":",
        linewidth=1.2,
        alpha=0.8,
        dashes=(1, 2),
        label="Travel",
    )
    ax.set_aspect("equal")
    cd = ops.cut_distance()
    title = (
        f"Adaptive Clearing — constant engagement\nCut distance: {cd:.1f} mm"
    )
    if ops.len() > 0:
        title += f"  |  {len(_ops_to_points(ops))} path points"
    ax.set_title(title)
    ax.legend(loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")

    return fig


# Re-use _ops_to_points for the title count
def _ops_to_points(ops):
    out = []
    for i in range(ops.len()):
        if ops.is_cutting(i) or ops.is_travel(i):
            ep = ops.endpoint(i)
            out.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return out


__docs_target__ = ["raygeo.ops.assembly.adaptive.md"]
__images__ = [
    {
        "heading": "adaptive_clearing",
        "caption": (
            "Forward-stepping constant-engagement clearing cuts "
            "(coloured by progress via the full-spectrum turbo gradient) "
            "from a central seed clearing (green), with MAT-routed "
            "travel links (red dashed) between segments."
        ),
        "function": generate_adaptive_clearing_demo,
    },
]

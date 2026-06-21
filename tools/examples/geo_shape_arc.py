"""Generate arc linearization example images."""

import math

import matplotlib.pyplot as plt

from raygeo.geo import Arc, Geometry
from raygeo.geo.shape.arc import (
    arc_through_point,
    get_polyline_turn_sign,
    linearize_arc,
)
from tools.plot import plot_geometry


def generate_linearize():
    r = 8
    arc_angle = 270
    sweep_rad = math.radians(arc_angle)
    end_x = r * math.cos(sweep_rad)
    end_y = r * math.sin(sweep_rad)

    geom = Geometry()
    geom.move_to(r, 0, 0)
    geom.arc_to(end_x, end_y, -r, 0, False, 0)

    cmds = geom.iter_typed_commands()
    arc_cmd = None
    for cmd in cmds:
        if isinstance(cmd, Arc):
            arc_cmd = cmd
            break

    fig, axes = plt.subplots(1, 3, figsize=(18, 6))

    plot_geometry(axes[0], geom, color="steelblue", linewidth=2)
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)
    axes[0].set_title(f"Original arc ({arc_angle}°)", fontsize=14)
    axes[0].set_xlim(-12, 12)
    axes[0].set_ylim(-12, 12)

    coarse_segments = linearize_arc(arc_cmd, (r, 0.0, 0.0), 4)
    for (sx, sy, _), (ex, ey, _) in coarse_segments:
        axes[1].plot(
            [sx, ex],
            [sy, ey],
            color="tomato",
            linewidth=2,
        )

    fine_segments = linearize_arc(arc_cmd, (r, 0.0, 0.0), 2)
    for (sx, sy, _), (ex, ey, _) in fine_segments:
        axes[2].plot(
            [sx, ex],
            [sy, ey],
            color="forestgreen",
            linewidth=2,
        )

    for ax in axes[1:]:
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_xlim(-12, 12)
        ax.set_ylim(-12, 12)
    axes[1].set_title("Coarse (res=4)", fontsize=14)
    axes[2].set_title("Fine (res=2)", fontsize=14)

    fig.tight_layout()
    return fig


def generate_arc_through_point():
    """Construct an arc through three points around a centre."""
    r = 5.0
    t_start = (r, 0.0)
    t_end = (0.0, r)
    t_mid = (r * 0.7071, r * 0.7071)
    center = (0.0, 0.0)
    arc = arc_through_point(t_start, t_end, t_mid, center, r)

    fig, ax = plt.subplots(figsize=(7, 7))
    xs = [p[0] for p in arc]
    ys = [p[1] for p in arc]
    ax.plot(
        xs,
        ys,
        "-o",
        color="steelblue",
        lw=2.5,
        markerfacecolor="lightblue",
        markeredgecolor="steelblue",
        markersize=4,
        label="Arc",
    )

    ax.plot(
        center[0], center[1], "x", color="gray", markersize=10, label="Centre"
    )
    ax.plot(
        t_start[0], t_start[1], "o", color="k", markersize=10, label="Start"
    )
    ax.plot(
        t_end[0], t_end[1], "s", color="tomato", markersize=10, label="End"
    )
    ax.plot(
        t_mid[0],
        t_mid[1],
        "*",
        color="gold",
        markersize=14,
        label="Mid (pass-through)",
    )

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_xlim(-r * 1.3, r * 1.3)
    ax.set_ylim(-r * 1.3, r * 1.3)
    ax.set_title(f"Arc through point (r={r})", fontsize=14)
    ax.legend(fontsize=11)
    fig.tight_layout()
    return fig


def generate_polyline_turn_sign():
    """Polyline turn sign."""
    ccw = [(10, 50), (50, 10), (90, 50)]
    cw = [(10, 30), (50, 70), (90, 30)]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

    for ax, polyline, title, expected in [
        (ax1, ccw, "CCW (left turn)", 1.0),
        (ax2, cw, "CW (right turn)", -1.0),
    ]:
        arr = [(float(x), float(y)) for x, y in polyline]
        xs = [p[0] for p in arr]
        ys = [p[1] for p in arr]

        ax.plot(xs, ys, "-o", color="steelblue", lw=2.5, ms=8)
        ax.plot(xs[0], ys[0], "o", color="green", ms=10, label="Start")
        ax.plot(xs[-1], ys[-1], "s", color="tomato", ms=10, label="End")

        sign = get_polyline_turn_sign(arr)
        arrow = "←" if sign < 0 else "→"
        ax.set_title(f"{title}  ({arrow} {sign:+.0f})", fontsize=13)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=10)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.shape.arc.md"]
__images__ = [
    {
        "heading": "arc_through_point",
        "caption": "Construct a circular arc through a given point",
        "function": generate_arc_through_point,
    },
    {
        "heading": "linearize_arc",
        "caption": "Arc linearization: coarse and fine resolution",
        "function": generate_linearize,
    },
    {
        "heading": "get_polyline_turn_sign",
        "caption": (
            "Determine turn direction of a polyline at its midpoint vertex"
        ),
        "function": generate_polyline_turn_sign,
    },
]

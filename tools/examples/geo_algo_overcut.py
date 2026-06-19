"""Generate overcut example images."""

import math

import matplotlib.pyplot as plt

from raygeo.geo import Geometry
from raygeo.geo.algo.overcut import apply_overcut
from tools.plot import plot_geometry


def _make_circle_polygon(cx, cy, r, n=128):
    pts = [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]
    return Geometry.from_points(pts, close=True)


def generate_overcut():
    cx, cy, r = 50, 50, 30
    geom = _make_circle_polygon(cx, cy, r)
    overcut_dist = 15.0
    result = apply_overcut(geom, overcut_dist)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_geometry(ax, geom, color="steelblue", linewidth=4, label="Original")
    plot_geometry(
        ax, result, color="forestgreen", linewidth=2, label="With overcut"
    )

    start = geom.iter_typed_commands()[0].end
    ax.plot(start[0], start[1], "o", color="gold", markersize=10, zorder=5)
    ax.plot(start[0], start[1], "o", color="black", markersize=4, zorder=6)

    orig_count = len(geom.iter_typed_commands())
    result_cmds = result.iter_typed_commands()
    overcut_cmds = result_cmds[orig_count:]
    if overcut_cmds:
        prev = result_cmds[orig_count - 1].end
        for i, cmd in enumerate(overcut_cmds):
            end = cmd.end
            ax.plot(
                [prev[0], end[0]],
                [prev[1], end[1]],
                color="crimson",
                linewidth=3,
                label="Overcut extension" if i == 0 else None,
            )
            prev = end
        last = overcut_cmds[-1].end
        ax.annotate(
            "Extended past start",
            xy=(last[0], last[1]),
            xytext=(last[0] - 15, last[1] - 22),
            arrowprops=dict(arrowstyle="->", color="crimson", linewidth=2),
            fontsize=10,
            color="crimson",
            fontweight="bold",
            ha="center",
        )

    ax.set_aspect("equal")
    ax.set_xlim(0, 115)
    ax.set_ylim(0, 100)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10, loc="upper right")
    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": "apply_overcut",
        "caption": "Overcut on closed contour",
        "function": generate_overcut,
    },
]

"""Generate polyline (motion assembly) example images."""

import matplotlib.pyplot as plt

from raygeo.ops.assembly.polyline import polyline_to_ops
from raygeo.ops.types import CommandType


def _plot_ops_2d(
    ax, ops, cut_color="steelblue", travel_color="darkorange", linewidth=2
):
    """Render Ops in 2D with visible travel lines."""
    ops.preload_state()
    last = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.SET_POWER:
            continue
        ep = ops.endpoint(i)
        if ct == CommandType.MOVE_TO:
            ax.plot(
                [last[0], ep[0]],
                [last[1], ep[1]],
                color=travel_color,
                linewidth=linewidth * 0.6,
                linestyle="--",
                alpha=0.85,
            )
            last = ep
        elif ct == CommandType.LINE_TO:
            ax.plot(
                [last[0], ep[0]],
                [last[1], ep[1]],
                color=cut_color,
                linewidth=linewidth,
            )
            last = ep


def generate_to_ops():
    """Polyline to ops."""
    pts: list[tuple[float, float, float]] = [
        (10.0, 10.0, 0.0),
        (50.0, 10.0, 0.0),
        (50.0, 40.0, -2.0),
        (10.0, 40.0, -2.0),
        (10.0, 10.0, 0.0),
    ]

    fig, axes = plt.subplots(1, 2, figsize=(12, 5))

    # Left: move_first=True
    ax = axes[0]
    ops_move = polyline_to_ops(pts, move_first=True)
    _plot_ops_2d(ax, ops_move, linewidth=2)
    ax.scatter(
        [p[0] for p in pts],
        [p[1] for p in pts],
        c="crimson",
        s=30,
        zorder=5,
        label="Input geometry (points)",
    )
    ax.plot([], [], color="steelblue", linewidth=2, label="Ops (cut → LineTo)")
    ax.plot(
        [],
        [],
        color="darkorange",
        linewidth=1.2,
        linestyle="--",
        label="Ops (travel → MoveTo)",
    )
    ax.set_aspect("equal")
    ax.set_xlim(0, 65)
    ax.set_ylim(0, 55)
    ax.set_title("polyline_to_ops(move_first=True)")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=7, loc="lower right")

    # Right: move_first=False
    ax = axes[1]
    ops_no_move = polyline_to_ops(pts, move_first=False)
    _plot_ops_2d(ax, ops_no_move, linewidth=2)
    ax.scatter(
        [p[0] for p in pts],
        [p[1] for p in pts],
        c="crimson",
        s=30,
        zorder=5,
        label="Input geometry (points)",
    )
    ax.plot([], [], color="steelblue", linewidth=2, label="Ops (cut → LineTo)")
    ax.set_aspect("equal")
    ax.set_xlim(0, 65)
    ax.set_ylim(0, 55)
    ax.set_title("polyline_to_ops(move_first=False)")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=7, loc="lower right")

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.polyline.md"]
__images__ = [
    {
        "heading": "polyline_to_ops",
        "caption": "polyline_to_ops with move_first=True vs move_first=False",
        "function": generate_to_ops,
    },
]

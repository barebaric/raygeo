"""Generate polyline (motion assembly) example images."""

import matplotlib.pyplot as plt

import raygeo.ops as ops_mod
from raygeo.ops.polyline import (
    LinkStrategy,
    find_pass_entry,
    find_pass_exit,
    link_passes,
    polyline_to_ops,
)
from raygeo.ops.types import CommandType


def _plot_ops_3d(
    ax, ops, cut_color="steelblue", travel_color="darkorange", linewidth=2
):
    """Render Ops in 3D with visible travel lines."""
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
                [last[2], ep[2]],
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
                [last[2], ep[2]],
                color=cut_color,
                linewidth=linewidth,
            )
            last = ep


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


def _rect_pass(x0, y0, w, h, z):
    """Build a rectangular pass as an Ops."""
    pts = [
        (float(x0), float(y0), float(z)),
        (float(x0 + w), float(y0), float(z)),
        (float(x0 + w), float(y0 + h), float(z)),
        (float(x0), float(y0 + h), float(z)),
        (float(x0), float(y0), float(z)),
    ]
    return polyline_to_ops(pts, move_first=True)


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


def generate_link_passes():
    """Link passes."""
    passes = [
        _rect_pass(10, 50, 30, 20, 0),
        _rect_pass(50, 50, 30, 20, -2),
        _rect_pass(90, 50, 30, 20, -4),
    ]

    fig = plt.figure(figsize=(14, 6))
    safe_z = 5.0

    for idx, (label, strategy) in enumerate(
        [
            ("StayDown", LinkStrategy.STAY_DOWN),
            ("Retract (safe_z=5)", LinkStrategy.RETRACT),
        ]
    ):
        ax = fig.add_subplot(1, 2, idx + 1, projection="3d")
        linked = link_passes(passes, safe_z, strategy)
        _plot_ops_3d(ax, linked, linewidth=2)
        for i, p in enumerate(passes):
            enter = p.endpoint(0)
            ax.plot(
                [enter[0]],
                [enter[1]],
                [enter[2]],
                "o",
                color="crimson",
                markersize=6,
            )
            ax.text(enter[0], enter[1], enter[2], f"P{i + 1}", fontsize=9)
        ax.plot(
            [],
            [],
            [],
            color="steelblue",
            linewidth=2,
            label="Ops (cut → LineTo)",
        )
        ax.plot(
            [],
            [],
            [],
            color="darkorange",
            linewidth=1.2,
            linestyle="--",
            label="Ops (travel → MoveTo)",
        )
        ax.set_xlim(0, 135)
        ax.set_ylim(40, 85)
        ax.set_zlim(-6, 7)
        ax.set_title(f"link_passes — {label}")
        ax.set_xlabel("X")
        ax.set_ylabel("Y")
        ax.set_zlabel("Z")
        ax.view_init(elev=25, azim=-40)
        ax.legend(fontsize=7, loc="upper left")

    fig.tight_layout()
    return fig


def generate_pass_entry_exit():
    """Pass entry exit."""
    fig, ax = plt.subplots(figsize=(8, 5))

    ops = ops_mod.Ops()
    ops.move_to(10.0, 10.0, 0.0)
    ops.line_to(50.0, 10.0, 0.0)
    ops.line_to(50.0, 40.0, -2.0)
    ops.line_to(10.0, 40.0, -2.0)

    _plot_ops_2d(ax, ops, linewidth=2)
    entry = find_pass_entry(ops)
    if entry:
        ax.scatter(
            entry[0], entry[1], c="forestgreen", s=120, zorder=6, label="Entry"
        )
    exit_ = find_pass_exit(ops)
    if exit_:
        ax.scatter(
            exit_[0], exit_[1], c="crimson", s=120, zorder=6, label="Exit"
        )

    ax.plot([], [], color="steelblue", linewidth=2, label="Cut (LineTo)")
    ax.plot(
        [],
        [],
        color="darkorange",
        linewidth=1.2,
        linestyle="--",
        label="Travel (MoveTo)",
    )
    ax.set_aspect("equal")
    ax.set_xlim(0, 65)
    ax.set_ylim(0, 55)
    ax.set_title("find_pass_entry / find_pass_exit")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=8, loc="lower right")

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.polyline.md"]
__images__ = [
    {
        "heading": "polyline_to_ops",
        "caption": "polyline_to_ops with move_first=True vs move_first=False",
        "function": generate_to_ops,
    },
    {
        "heading": "link_passes",
        "caption": "Three passes linked with StayDown vs Retract strategies",
        "function": generate_link_passes,
    },
    {
        "heading": "find_pass_entry",
        "caption": (
            "Entry and exit points from find_pass_entry / find_pass_exit"
        ),
        "function": generate_pass_entry_exit,
    },
]

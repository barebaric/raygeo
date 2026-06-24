"""Generate merge_overlapping_lines example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType


def _plot_merged(ax, orig, ops, title):
    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if pos != ep:
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot([], [], color="forestgreen", linewidth=2.5, label="Merged")
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    ax.set_title(title)
    xl = ax.get_xlim()
    yl = ax.get_ylim()
    if abs(yl[1] - yl[0]) < 1:
        pad = max(abs(xl[1] - xl[0]) * 0.1, 5.0)
        ax.set_ylim(-pad, pad)


def generate_merge_lines():
    presets = [
        ("Near-duplicate lines", "nd"),
        ("Adjacent rectangles", "adj"),
    ]

    fig3, axes3 = plt.subplots(1, 2, figsize=(16, 6))

    for idx, (name, key) in enumerate(presets):
        ops3 = Ops()
        ops3.set_power(1.0)
        if key == "nd":
            ops3.move_to(0, 0)
            ops3.line_to(100, 0)
            ops3.move_to(0, 1.5)
            ops3.line_to(100, 1.5)
            ops3.move_to(0, 5)
            ops3.line_to(100, 5)
            tol = 2.0
        else:
            ops3.move_to(0, 0)
            ops3.line_to(100, 0)
            ops3.line_to(100, 100)
            ops3.line_to(0, 100)
            ops3.line_to(0, 0)
            ops3.move_to(100, 0)
            ops3.line_to(200, 0)
            ops3.line_to(200, 100)
            ops3.line_to(100, 100)
            ops3.line_to(100, 0)
            tol = 1.0

        orig3 = ops3.copy()
        ops3.merge_overlapping_lines(tol)
        _plot_merged(axes3[idx], orig3, ops3, f"{name} (tol={tol})")

    fig3.tight_layout()
    return fig3


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "merge_overlapping_lines",
        "caption": "Line merging before and after",
        "function": generate_merge_lines,
    },
]

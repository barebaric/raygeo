"""Generate apply_drag_knife example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def _plot_ops(ax, ops, linewidth=2.5):
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
        if ct in (CommandType.LINE_TO, CommandType.ARC_TO):
            ep = ops.endpoint(i)
            state = ops.state(i)
            color = (
                "dodgerblue" if state and state.power < 0.01 else "forestgreen"
            )
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=color,
                linewidth=linewidth,
                solid_capstyle="round",
            )
            pos = ep


def generate_drag_knife():
    offset = 4.0

    ops = Ops()
    ops.set_power(1.0)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(20, 20, 0)
    ops.line_to(80, 20, 0)
    ops.line_to(80, 80, 0)
    ops.line_to(20, 80, 0)
    ops.line_to(20, 20, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig = ops.copy()
    ops.apply_drag_knife(offset, 90.0)

    fig, ax = plt.subplots(figsize=(10, 10))

    _plot_ops(ax, orig, linewidth=6)
    _plot_ops(ax, ops)

    ax.plot(
        [],
        [],
        color="tomato",
        linewidth=6,
        alpha=0.35,
        label="Desired cut path",
    )
    ax.plot(
        [],
        [],
        color="forestgreen",
        linewidth=2.5,
        label="Pivot path (blade down)",
    )
    ax.plot([], [], color="dodgerblue", linewidth=2.5, label="Blade up")
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.plot(20, 20, "o", color="black", markersize=6)
    ax.annotate(
        "pivot arcs around corner",
        (80, 20),
        (92, 8),
        fontsize=9,
        arrowprops={"arrowstyle": "->"},
    )
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_drag_knife",
        "caption": "Drag-knife pivot path around a square contour",
        "function": generate_drag_knife,
    },
]

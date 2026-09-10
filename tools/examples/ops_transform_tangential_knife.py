"""Generate apply_tangential_knife example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def generate_tangential_knife():
    ops = Ops()
    ops.set_power(1.0)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(10, 20, 0)
    ops.line_to(80, 20, 0)
    ops.line_to(80, 60, 0)
    ops.line_to(30, 60, 0)
    ops.line_to(30, 90, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig = ops.copy()
    ops.apply_tangential_knife(45.0, 1.0, 8.0)

    fig, ax = plt.subplots(figsize=(10, 8))

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
                linewidth=6,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct not in (CommandType.MOVE_TO, CommandType.LINE_TO):
            continue
        ep = ops.endpoint(i)
        color = "dodgerblue" if ep[2] > 1e-9 else "forestgreen"
        ax.plot(
            [pos[0], ep[0]],
            [pos[1], ep[1]],
            color=color,
            linewidth=2.5,
            solid_capstyle="round",
        )
        pos = ep

    ax.annotate(
        "lift, rotate A, plunge",
        (80.0, 20.0),
        (88.0, 4.0),
        fontsize=9,
        arrowprops={"arrowstyle": "->"},
    )

    ax.plot(
        [],
        [],
        color="tomato",
        linewidth=6,
        alpha=0.35,
        label="Desired cut path",
    )
    ax.plot(
        [], [], color="forestgreen", linewidth=2.5, label="Cut (knife down)"
    )
    ax.plot(
        [],
        [],
        color="dodgerblue",
        linewidth=2.5,
        label="Lift / rotate (safe Z)",
    )
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10, loc="lower left")

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_tangential_knife",
        "caption": "Tangential knife path with A-axis rotation lifts",
        "function": generate_tangential_knife,
    },
]

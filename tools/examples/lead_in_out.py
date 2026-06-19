"""Generate lead-in/lead-out example images."""

__images__ = [
    {
        "stem": "lead-in-out",
        "caption": "Lead-in and lead-out paths",
        "doc": "raygeo.ops.md",
        "heading": "apply_lead_in_out",
    },
]

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def generate_examples(output_dir):
    images = []

    lead_in = 5.0
    lead_out = 5.0

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
    ops.apply_lead_in_out(lead_in, lead_out)

    fig, ax = plt.subplots(figsize=(10, 10))

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
            state = ops.state(i)
            color = (
                "dodgerblue" if state and state.power < 0.01 else "forestgreen"
            )
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=color,
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot(
        [], [], color="forestgreen", linewidth=2.5, label="Cut (power > 0)"
    )
    ax.plot(
        [], [], color="dodgerblue", linewidth=2.5, label="Lead (power = 0)"
    )
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)

    fig.tight_layout()
    path = output_dir / "lead-in-out.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "lead-in-out.png",
            "caption": "Lead-in and lead-out paths on a rectangle",
        }
    )

    return {
        "title": "Lead-In / Lead-Out",
        "description": (
            "Add lead-in and lead-out paths to vector outlines for cutting."
        ),
        "images": images,
    }

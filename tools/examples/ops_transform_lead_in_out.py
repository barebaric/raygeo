"""Generate apply_lead_in_out example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def generate_lead_in_out():
    lead_in = 5.0
    lead_out = 5.0

    ops2 = Ops()
    ops2.set_power(1.0)
    ops2.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops2.move_to(20, 20, 0)
    ops2.line_to(80, 20, 0)
    ops2.line_to(80, 80, 0)
    ops2.line_to(20, 80, 0)
    ops2.line_to(20, 20, 0)
    ops2.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig2 = ops2.copy()
    ops2.apply_lead_in_out(lead_in, lead_out)

    fig2, ax_lead = plt.subplots(figsize=(10, 10))

    orig2.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig2.len()):
        ct = orig2.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig2.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig2.endpoint(i)
            ax_lead.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops2.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops2.len()):
        ct = ops2.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops2.endpoint(i)
            if pos != ep:
                ax_lead.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops2.endpoint(i)
            state = ops2.state(i)
            color = (
                "dodgerblue" if state and state.power < 0.01 else "forestgreen"
            )
            ax_lead.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=color,
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax_lead.plot(
        [], [], color="tomato", linewidth=5, alpha=0.35, label="Original"
    )
    ax_lead.plot(
        [], [], color="forestgreen", linewidth=2.5, label="Cut (power > 0)"
    )
    ax_lead.plot(
        [], [], color="dodgerblue", linewidth=2.5, label="Lead (power = 0)"
    )
    ax_lead.plot(
        [], [], color="gray", linewidth=0.7, linestyle=":", label="Travel"
    )
    ax_lead.set_aspect("equal")
    ax_lead.grid(True, alpha=0.3)
    ax_lead.legend(fontsize=10)

    fig2.tight_layout()
    return fig2


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_lead_in_out",
        "caption": "Lead-in and lead-out paths",
        "function": generate_lead_in_out,
    },
]

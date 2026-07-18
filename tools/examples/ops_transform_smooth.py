"""Generate smooth example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def _plot(ax, seq, title):
    seq.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(seq.len()):
        ct = seq.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = seq.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = seq.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="steelblue",
                linewidth=3,
                solid_capstyle="round",
            )
            pos = ep
    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.grid(True, alpha=0.3)
    ax.set_title(title, fontsize=12)


def generate_smooth():
    ops = Ops()
    ops.set_power(1.0)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(20, 20, 0)
    ops.line_to(80, 20, 0)
    ops.line_to(80, 80, 0)
    ops.line_to(20, 80, 0)
    ops.close_path()
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig = ops.copy()
    smooth_med = ops.copy()
    smooth_high = ops.copy()
    smooth_med.smooth(40, 80.0)
    smooth_high.smooth(90, 80.0)

    fig, (ax1, ax2, ax3) = plt.subplots(1, 3, figsize=(18, 5.5))
    _plot(ax1, orig, "Original")
    _plot(ax2, smooth_med, "Smooth (amount=40)")
    _plot(ax3, smooth_high, "Smooth (amount=90)")
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "smooth",
        "caption": "Gaussian smoothing applied to a square path",
        "function": generate_smooth,
    },
]

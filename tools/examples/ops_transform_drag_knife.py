"""Generate apply_drag_knife example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import SectionType
from tools.plot import plot_ops_2d


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

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))
    plot_ops_2d(ax1, orig, mark_start=False, mark_end=False)
    ax1.set_title("Desired cut path", fontsize=13)
    plot_ops_2d(ax2, ops, mark_start=False, mark_end=False)
    ax2.set_title(f"Drag-knife pivot path ({offset} mm offset)", fontsize=13)
    for ax in (ax1, ax2):
        ax.set_xlim(0, 105)
        ax.set_ylim(0, 100)
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

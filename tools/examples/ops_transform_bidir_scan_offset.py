"""Generate apply_bidir_scan_offset example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from tools.plot import plot_ops_2d


def generate_bidir_scan_offset():
    ops = Ops()
    ops.set_power(1.0)
    ops.move_to(10, 10, 0)
    ops.scan_to(90, 10, 0)
    ops.move_to(90, 20, 0)
    ops.scan_to(10, 20, 0)
    ops.move_to(10, 30, 0)
    ops.scan_to(90, 30, 0)
    ops.move_to(90, 40, 0)
    ops.scan_to(10, 40, 0)
    ops.move_to(10, 50, 0)
    ops.scan_to(90, 50, 0)
    ops.move_to(90, 60, 0)
    ops.scan_to(10, 60, 0)

    offset = 2.0
    orig = ops.copy()
    ops.apply_bidir_scan_offset(offset)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))
    plot_ops_2d(ax1, orig, mark_start=False, mark_end=False)
    ax1.set_title("Original", fontsize=13)
    plot_ops_2d(ax2, ops, mark_start=False, mark_end=False)
    ax2.set_title(f"Offset applied ({offset} mm)", fontsize=13)
    for ax in (ax1, ax2):
        ax.set_xlim(0, 105)
        ax.set_ylim(0, 75)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=10)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_bidir_scan_offset",
        "caption": "Bidirectional scan offset correction",
        "function": generate_bidir_scan_offset,
    },
]

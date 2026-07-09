"""Generate apply_tab_gaps example images."""

import math

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import SectionType
from tools.plot import plot_ops_2d


def generate_tab_operations():
    cx, cy = 10, 10
    w, h = 20, 20

    ops6 = Ops()
    ops6.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops6.set_power(1.0)
    ops6.move_to(cx - w / 2, cy - h / 2, 0)
    ops6.line_to(cx + w / 2, cy - h / 2, 0)
    ops6.line_to(cx + w / 2, cy + h / 2, 0)
    ops6.line_to(cx - w / 2, cy + h / 2, 0)
    ops6.close_path()
    ops6.ops_section_end(SectionType.VECTOR_OUTLINE)

    geo = ops6.to_geometry()
    segments = geo.segments()
    seg_dists = []
    for seg in segments:
        for j in range(1, len(seg)):
            dx = seg[j][0] - seg[j - 1][0]
            dy = seg[j][1] - seg[j - 1][1]
            seg_dists.append(math.sqrt(dx * dx + dy * dy))
    total_dist = sum(seg_dists)

    n_tabs = 2
    tab_width = 2.0
    clips = []
    if total_dist > 0 and n_tabs > 0:
        flat_pts = [p for seg in segments for p in seg]
        for t in range(n_tabs):
            target = total_dist * (t + 1) / (n_tabs + 1)
            accum = 0.0
            for seg_i, sd in enumerate(seg_dists):
                if accum + sd >= target - 1e-9:
                    frac = (target - accum) / sd if sd > 1e-9 else 0.0
                    px = flat_pts[seg_i][0] + frac * (
                        flat_pts[seg_i + 1][0] - flat_pts[seg_i][0]
                    )
                    py = flat_pts[seg_i][1] + frac * (
                        flat_pts[seg_i + 1][1] - flat_pts[seg_i][1]
                    )
                    clips.append((px, py, tab_width))
                    break
                accum += sd

    result_ops = ops6.copy()
    if clips:
        result_ops.apply_tab_gaps(clips)

    fig6, axes6 = plt.subplots(1, 2, figsize=(14, 6))
    axes6[0].set_title("Original")
    plot_ops_2d(axes6[0], ops6)
    for cx_, cy_, tw_ in clips:
        axes6[0].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
        axes6[0].add_patch(
            mpatches.Circle(
                (cx_, cy_),
                tw_ / 2,
                fill=False,
                color="red",
                linestyle="--",
                linewidth=1,
            )
        )
    axes6[0].set_aspect("equal")
    axes6[0].grid(True, alpha=0.3)

    axes6[1].set_title("After Gap Tabs")
    plot_ops_2d(axes6[1], result_ops)
    for cx_, cy_, tw_ in clips:
        axes6[1].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
    axes6[1].set_aspect("equal")
    axes6[1].grid(True, alpha=0.3)

    fig6.tight_layout()
    return fig6


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_tab_gaps",
        "caption": "Tab operations on a rectangle",
        "function": generate_tab_operations,
    },
]

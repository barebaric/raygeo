"""Generate tab operations example images."""

import math

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import SectionType
from tools.plot import plot_ops


def generate_examples(output_dir):
    images = []
    cx, cy = 10, 10
    w, h = 20, 20

    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.set_power(1.0)
    ops.move_to(cx - w / 2, cy - h / 2, 0)
    ops.line_to(cx + w / 2, cy - h / 2, 0)
    ops.line_to(cx + w / 2, cy + h / 2, 0)
    ops.line_to(cx - w / 2, cy + h / 2, 0)
    ops.close_path()
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    geo = ops.to_geometry()
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

    result_ops = ops.copy()
    if clips:
        result_ops.apply_tab_gaps(clips)

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    axes[0].set_title("Original")
    plot_ops(axes[0], ops, color="steelblue")
    for cx_, cy_, tw_ in clips:
        axes[0].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
        axes[0].add_patch(
            mpatches.Circle(
                (cx_, cy_),
                tw_ / 2,
                fill=False,
                color="red",
                linestyle="--",
                linewidth=1,
            )
        )
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)

    axes[1].set_title("After Gap Tabs")
    plot_ops(axes[1], result_ops, color="steelblue")
    for cx_, cy_, tw_ in clips:
        axes[1].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)

    fig.tight_layout()
    path = output_dir / "tab-operations.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "tab-operations.png",
            "caption": "Tab operations: gap tabs on a rectangle",
        }
    )

    return {
        "title": "Tab Operations",
        "description": (
            "Apply tab gaps or tab power modulation to vector outlines."
        ),
        "images": images,
    }

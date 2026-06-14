"""Generate ops clipping example images."""

import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

from raygeo.ops import Ops
from raygeo.ops.types import CommandType


def generate_examples(output_dir):
    images = []

    ops = Ops()
    ops.set_power(1.0)
    ops.move_to(10, 10, 0)
    ops.line_to(90, 10, 0)
    ops.line_to(90, 90, 0)
    ops.line_to(10, 90, 0)
    ops.line_to(10, 10, 0)
    ops.move_to(30, 30, 0)
    ops.line_to(70, 30, 0)
    ops.line_to(70, 70, 0)
    ops.line_to(30, 70, 0)
    ops.line_to(30, 30, 0)
    ops.move_to(20, 40, 0)
    ops.line_to(80, 40, 0)
    ops.line_to(50, 80, 0)
    ops.line_to(20, 40, 0)

    clip_rect = (25.0, 25.0, 75.0, 85.0)

    clipped = ops.clip_rect(clip_rect)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))

    def _plot_ops(ax, seq, title):
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
                    linewidth=2.5,
                    solid_capstyle="round",
                )
                pos = ep
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3)
        ax.set_title(title, fontsize=13)

    _plot_ops(ax1, ops, "Original paths")
    ax1.add_patch(
        Rectangle(
            (clip_rect[0], clip_rect[1]),
            clip_rect[2] - clip_rect[0],
            clip_rect[3] - clip_rect[1],
            fill=False,
            edgecolor="tomato",
            linewidth=2,
            linestyle="--",
            label="Clip rect",
        )
    )
    ax1.legend(fontsize=10)

    _plot_ops(ax2, clipped, "After clip_rect")
    ax2.add_patch(
        Rectangle(
            (clip_rect[0], clip_rect[1]),
            clip_rect[2] - clip_rect[0],
            clip_rect[3] - clip_rect[1],
            fill=False,
            edgecolor="tomato",
            linewidth=2,
            linestyle="--",
            label="Clip rect",
        )
    )
    ax2.legend(fontsize=10)

    fig.tight_layout()
    path = output_dir / "ops-clip-rect.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "ops-clip-rect.png",
            "caption": "Ops paths clipped to a rectangle",
        }
    )

    return {
        "title": "Ops Clipping",
        "description": "Clip Ops paths to rectangles and polygonal regions.",
        "images": images,
    }

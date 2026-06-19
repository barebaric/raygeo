"""Generate overscan example images."""

__images__ = [
    {
        "stem": "overscan",
        "caption": "Overscan applied to raster lines",
        "doc": "raygeo.ops.md",
        "heading": "apply_overscan",
    },
]

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def generate_examples(output_dir):
    images = []

    ops = Ops()
    ops.set_power(1.0)
    ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
    ops.move_to(10, 10, 0)
    ops.line_to(90, 10, 0)
    ops.move_to(10, 20, 0)
    ops.line_to(90, 20, 0)
    ops.move_to(10, 30, 0)
    ops.line_to(90, 30, 0)
    ops.ops_section_end(SectionType.RASTER_FILL)

    dist = 5.0
    orig = ops.copy()
    ops.apply_overscan(dist)

    fig, ax = plt.subplots(figsize=(12, 8))

    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct in (CommandType.LINE_TO, CommandType.SCAN_LINE):
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
        if ct in (CommandType.LINE_TO, CommandType.SCAN_LINE):
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot([], [], color="forestgreen", linewidth=2.5, label="With overscan")
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    ax.set_title(f"Overscan distance: {dist} mm")

    fig.tight_layout()
    path = output_dir / "overscan.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {"path": "overscan.png", "caption": "Overscan applied to raster lines"}
    )

    return {
        "title": "Overscan",
        "description": (
            "Extend raster and vector scan lines beyond the image boundary "
            "for clean edges."
        ),
        "images": images,
    }

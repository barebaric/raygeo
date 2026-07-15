"""Generate apply_overscan example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, RasterMode, SectionType


def generate_overscan():
    ops5 = Ops()
    ops5.set_power(1.0)
    ops5.ops_section_start(
        SectionType.RASTER_FILL, "wp1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops5.move_to(10, 10, 0)
    ops5.line_to(90, 10, 0)
    ops5.move_to(10, 20, 0)
    ops5.line_to(90, 20, 0)
    ops5.move_to(10, 30, 0)
    ops5.line_to(90, 30, 0)
    ops5.ops_section_end(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    dist = 5.0
    orig5 = ops5.copy()
    ops5.apply_overscan(dist)

    fig5, ax_scan = plt.subplots(figsize=(12, 8))

    orig5.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig5.len()):
        ct = orig5.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig5.endpoint(i)
            continue
        if ct in (CommandType.LINE_TO, CommandType.SCAN_LINE):
            ep = orig5.endpoint(i)
            ax_scan.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops5.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops5.len()):
        ct = ops5.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops5.endpoint(i)
            if pos != ep:
                ax_scan.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct in (CommandType.LINE_TO, CommandType.SCAN_LINE):
            ep = ops5.endpoint(i)
            ax_scan.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax_scan.plot(
        [], [], color="tomato", linewidth=5, alpha=0.35, label="Original"
    )
    ax_scan.plot(
        [], [], color="forestgreen", linewidth=2.5, label="With overscan"
    )
    ax_scan.plot(
        [], [], color="gray", linewidth=0.7, linestyle=":", label="Travel"
    )
    ax_scan.set_aspect("equal")
    ax_scan.grid(True, alpha=0.3)
    ax_scan.legend(fontsize=10)
    ax_scan.set_title(f"Overscan distance: {dist} mm")

    fig5.tight_layout()
    return fig5


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_overscan",
        "caption": "Overscan applied to raster lines",
        "function": generate_overscan,
    },
]

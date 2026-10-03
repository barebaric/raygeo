"""Generate apply_merge_scanlines example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, RasterMode, SectionType

CUT_SPEED = 6000.0
RAPID_SPEED = 12000.0
LOW_ACCEL = 100.0
HIGH_ACCEL = 20000.0


def _build_array():
    """Two full-sweep 'workpieces' side by side, serpentine rows.

    Mirrors the step-aggregate structure of an engraved array: each
    workpiece's raster is wrapped in its own section and markers,
    with a 6mm gap between the two elements.
    """
    ops = Ops()
    ops.set_feed_rate(CUT_SPEED)
    ops.set_rapid_rate(RAPID_SPEED)
    for name, x0 in (("wp-a", 10.0), ("wp-b", 56.0)):
        ops.workpiece_start(name)
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            name,
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        for r in range(6):
            y = 12.0 + r * 4.0
            values = [80 + 15 * j for j in range(9)]
            if r % 2 == 0:
                ops.move_to(x0, y, 0)
                ops.scan_to(x0 + 40, y, 0, power_values=values)
            else:
                ops.move_to(x0 + 40, y, 0)
                ops.scan_to(x0, y, 0, power_values=values)
        ops.ops_section_end(
            SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
        )
        ops.workpiece_end(name)
    return ops


def _plot(ax, seq, title, legend=False):
    seq.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(seq.len()):
        ct = seq.command_type(i)
        ep = seq.endpoint(i)
        if ct == CommandType.MOVE_TO:
            if pos != ep:
                ax.annotate(
                    "",
                    xy=(ep[0], ep[1]),
                    xytext=(pos[0], pos[1]),
                    arrowprops=dict(
                        arrowstyle="->", color="gray", lw=1.2, linestyle=":"
                    ),
                )
            pos = ep
            continue
        if ct == CommandType.SCAN_LINE:
            values = list(seq.scanline_data(i))
            n = len(values)
            for j, p in enumerate(values):
                t0 = j / n
                t1 = (j + 1) / n
                x0 = pos[0] + (ep[0] - pos[0]) * t0
                y0 = pos[1] + (ep[1] - pos[1]) * t0
                x1 = pos[0] + (ep[0] - pos[0]) * t1
                y1 = pos[1] + (ep[1] - pos[1]) * t1
                if p > 0:
                    ax.plot(
                        [x0, x1],
                        [y0, y1],
                        color="steelblue",
                        linewidth=3,
                        solid_capstyle="butt",
                    )
                else:
                    ax.plot(
                        [x0, x1],
                        [y0, y1],
                        color="darkorange",
                        linewidth=3,
                        linestyle=(0, (2, 2)),
                        solid_capstyle="butt",
                    )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="steelblue",
                linewidth=3,
                solid_capstyle="round",
            )
            pos = ep

    if legend:
        ax.plot(
            [], [], color="steelblue", linewidth=3, label="Engraved (power)"
        )
        ax.plot(
            [],
            [],
            color="darkorange",
            linewidth=3,
            linestyle=(0, (2, 2)),
            label="Bridged at zero power",
        )
        ax.plot(
            [], [], color="gray", linewidth=1.2, linestyle=":", label="Travel"
        )
        ax.legend(fontsize=9, loc="upper right")
    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(5, 42)
    ax.grid(True, alpha=0.3)
    ax.set_title(title, fontsize=11)


def _seconds(ops, accel):
    return ops.estimate_time(CUT_SPEED, RAPID_SPEED, accel)


def generate_apply_merge_scanlines():
    orig = _build_array()

    merged = orig.copy()
    merged.apply_merge_scanlines(LOW_ACCEL, CUT_SPEED, RAPID_SPEED)

    declined = orig.copy()
    declined.apply_merge_scanlines(HIGH_ACCEL, CUT_SPEED, RAPID_SPEED)

    t_before = _seconds(orig, LOW_ACCEL)
    t_after = _seconds(merged, LOW_ACCEL)
    saving = 100 * (t_before - t_after) / t_before

    fig, (ax1, ax2, ax3) = plt.subplots(1, 3, figsize=(22, 5))

    _plot(ax1, orig, "Before: per-workpiece sweeps", legend=True)
    _plot(
        ax2,
        merged,
        f"Merged (a = {LOW_ACCEL:.0f} mm/s²)"
        f"\n{t_before:.1f}s → {t_after:.1f}s (−{saving:.0f}%)",
    )
    _plot(
        ax3,
        declined,
        f"Declined (a = {HIGH_ACCEL:.0f} mm/s²)"
        f"\nstop/start is cheap; bridging the gap would lose time",
    )

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_merge_scanlines",
        "caption": (
            "Scanline merging across gaps: engaged at low acceleration, "
            "declined when it would lose time"
        ),
        "function": generate_apply_merge_scanlines,
    },
]

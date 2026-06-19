"""Generate rasterization example images."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import Normalize

from raygeo.ops.raster import (
    ScanMode,
    extract_zero_power_segments,
    rasterize_mask_lines,
    rasterize_mask_scan,
    rasterize_multi_pass,
    rasterize_power_modulation,
)
from raygeo.ops.types import CommandType
from tools.plot import make_pattern


def _plot_raster_result(gray, ops, title, img_size, ppm):
    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    max_mm = img_size / ppm
    axes[0].imshow(
        gray,
        cmap="gray",
        vmin=0,
        vmax=255,
        origin="lower",
        extent=[0, max_mm, 0, max_mm],
    )
    axes[0].set_title("Input image")
    axes[0].set_xlim(0, max_mm)
    axes[0].set_ylim(0, max_mm)
    axes[0].set_aspect("equal")

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    z_cycle = ["#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00"]
    power_cmap = plt.get_cmap("plasma")

    z_set: set[float] = set()
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct in (CommandType.LINE_TO, CommandType.SCAN_LINE):
            z_set.add(ops.endpoint(i)[2])
    multi_pass = len(z_set) > 1

    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if abs(pos[0] - ep[0]) > 1e-6 or abs(pos[1] - ep[1]) > 1e-6:
                axes[1].plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="#cccccc",
                    linewidth=0.5,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct in (
            CommandType.SET_POWER,
            CommandType.SET_FEED_RATE,
            CommandType.SET_RAPID_RATE,
            CommandType.SET_HEAD,
            CommandType.SET_FREQUENCY,
            CommandType.SET_PULSE_WIDTH,
            CommandType.SET_COOLANT,
        ):
            continue
        st = ops.state(i)
        pwr = st.power if st is not None and st.power is not None else 1.0
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            z_idx = hash(ep[2]) % len(z_cycle) if multi_pass else 0
            color = z_cycle[z_idx] if multi_pass else power_cmap(pwr)
            axes[1].plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=color,
                linewidth=1.2,
                alpha=0.4 + 0.6 * pwr
                if not multi_pass
                else min(1.0, 0.3 + pwr),
            )
            pos = ep
            continue
        if ct in (CommandType.ARC_TO, CommandType.BEZIER_TO):
            ep = ops.endpoint(i)
            axes[1].plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="#cccccc",
                linewidth=0.5,
                linestyle=":",
            )
            pos = ep
            continue
        if ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            z_idx = hash(ep[2]) % len(z_cycle) if multi_pass else 0
            base_color = z_cycle[z_idx] if multi_pass else None
            sd = ops.scanline_data(i)
            n = len(sd)
            if n > 0:
                xs = np.linspace(pos[0], ep[0], n)
                ys = np.linspace(pos[1], ep[1], n)
                power_arr = np.frombuffer(sd, dtype=np.uint8).astype(
                    np.float64
                )
                pn = power_arr / 255.0
                for j in range(n - 1):
                    color = base_color if multi_pass else power_cmap(pn[j])
                    alpha = 0.3 + 0.7 * pn[j] if multi_pass else 1.0
                    axes[1].plot(
                        xs[j : j + 2],
                        ys[j : j + 2],
                        color=color,
                        linewidth=1.2,
                        alpha=alpha,
                    )
            pos = ep
            continue

    axes[1].set_xlim(0, max_mm)
    axes[1].set_ylim(0, max_mm)
    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)
    axes[1].set_title(title)

    fig.tight_layout()
    return fig


def generate_power_modulation():
    """Power modulation."""
    img_size = 64
    gray = make_pattern(img_size, img_size, "Radial")
    ppm = 10.0
    line_interval = 0.1

    alpha = np.full((img_size, img_size), 255, dtype=np.uint8)
    ops = rasterize_power_modulation(
        gray,
        alpha,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval,
        0.05,
        min_power=0.0,
        max_power=1.0,
        angle=0,
        scan_mode=ScanMode.Segmented,
    )
    return _plot_raster_result(
        gray, ops, "Raster: Power Modulation", img_size, ppm
    )


def generate_mask_scan():
    """Mask scan."""
    img_size = 64
    gray = make_pattern(img_size, img_size, "Radial")
    ppm = 10.0
    line_interval = 0.1

    mask = (gray > 128).astype(np.uint8)
    ops = rasterize_mask_scan(
        mask,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval,
        angle=0,
        scan_mode=ScanMode.Segmented,
    )
    return _plot_raster_result(gray, ops, "Raster: Mask Scan", img_size, ppm)


def generate_mask_lines():
    """Mask lines."""
    img_size = 64
    gray = make_pattern(img_size, img_size, "Radial")
    ppm = 10.0
    line_interval = 0.1

    mask = (gray > 128).astype(np.uint8)
    ops = rasterize_mask_lines(
        mask,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval,
        angle=0,
        scan_mode=ScanMode.Segmented,
    )
    return _plot_raster_result(gray, ops, "Raster: Mask Lines", img_size, ppm)


def generate_multi_pass():
    """Multi pass."""
    img_size = 64
    gray = make_pattern(img_size, img_size, "Radial")
    ppm = 10.0
    line_interval = 0.1

    ops = rasterize_multi_pass(
        gray,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval,
        3,
        0.5,
        angle=0,
        scan_mode=ScanMode.Segmented,
    )
    return _plot_raster_result(gray, ops, "Raster: Multi-Pass", img_size, ppm)


def generate_zero_power_segments():
    """Zero power segments."""
    n_steps = 100

    power_values = np.full(n_steps, 200, dtype=np.uint8)
    power_values[15:30] = 0
    power_values[50:65] = 0
    power_values[80:90] = 0

    start = (0.0, 0.0, 0.0)
    end = (50.0, 30.0, 0.0)

    segments = extract_zero_power_segments(start, end, power_values.tobytes())
    seg_pts = np.array(segments).reshape(-1, 2, 3)

    xs = np.linspace(start[0], end[0], n_steps)
    ys = np.linspace(start[1], end[1], n_steps)

    fig2, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

    cmap = plt.get_cmap("RdYlGn")
    norm_power = power_values.astype(np.float64) / 255.0

    for i in range(n_steps - 1):
        ax1.plot(
            xs[i : i + 2],
            ys[i : i + 2],
            color=cmap(norm_power[i]),
            linewidth=3,
        )
    ax1.scatter(
        [start[0], end[0]], [start[1], end[1]], color="black", s=40, zorder=5
    )
    ax1.set_xlabel("X (mm)")
    ax1.set_ylabel("Y (mm)")
    ax1.set_title("Before: scanline colored by power")
    ax1.set_aspect("equal")
    ax1.grid(True, alpha=0.3)

    sm = plt.cm.ScalarMappable(cmap=cmap, norm=Normalize(0, 255))
    sm.set_array([])
    fig2.colorbar(sm, ax=ax1, label="Power", shrink=0.7)

    ax2.plot(xs, ys, color="lightgray", linewidth=2, label="Full scanline")
    for idx, seg in enumerate(seg_pts):
        ax2.plot(
            [seg[0, 0], seg[1, 0]],
            [seg[0, 1], seg[1, 1]],
            color="red",
            linewidth=4,
            label="Zero-power segment" if idx == 0 else "",
        )
    ax2.scatter(
        [start[0], end[0]], [start[1], end[1]], color="black", s=40, zorder=5
    )
    ax2.set_xlabel("X (mm)")
    ax2.set_ylabel("Y (mm)")
    ax2.set_title("After: extracted zero-power segments")
    ax2.set_aspect("equal")
    ax2.grid(True, alpha=0.3)
    ax2.legend()

    fig2.tight_layout()
    return fig2


__images__ = [
    {
        "heading": "rasterize_power_modulation",
        "caption": "Rasterization: Power Modulation",
        "function": generate_power_modulation,
    },
    {
        "heading": "rasterize_mask_scan",
        "caption": "Rasterization: Mask Scan",
        "function": generate_mask_scan,
    },
    {
        "heading": "rasterize_mask_lines",
        "caption": "Rasterization: Mask Lines",
        "function": generate_mask_lines,
    },
    {
        "heading": "rasterize_multi_pass",
        "caption": "Rasterization: Multi-Pass",
        "function": generate_multi_pass,
    },
    {
        "heading": "extract_zero_power_segments",
        "caption": "Zero-power segment extraction",
        "function": generate_zero_power_segments,
    },
]

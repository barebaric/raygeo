"""Generate rasterization example images."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.ops.raster import (
    ScanMode,
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


def generate_examples(output_dir):
    images = []
    img_size = 64
    gray = make_pattern(img_size, img_size, "Radial")
    ppm = 10.0
    line_interval = 0.1

    modes = []

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
    modes.append(("Mask Scan", ops))

    ops2 = rasterize_mask_lines(
        mask,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval,
        angle=0,
        scan_mode=ScanMode.Segmented,
    )
    modes.append(("Mask Lines", ops2))

    ops3 = rasterize_multi_pass(
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
    modes.append(("Multi-Pass", ops3))

    alpha = np.full((img_size, img_size), 255, dtype=np.uint8)
    ops4 = rasterize_power_modulation(
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
    modes.append(("Power Modulation", ops4))

    for name, ops in modes:
        fig = _plot_raster_result(gray, ops, f"Raster: {name}", img_size, ppm)
        fname = f"rasterization-{name.lower().replace(' ', '-')}.png"
        fig.savefig(output_dir / fname, dpi=150)
        plt.close(fig)
        images.append(
            {"path": fname, "caption": f"Rasterization mode: {name}"}
        )

    return {
        "title": "Rasterization",
        "description": (
            "Rasterize images into CNC control sequences using various "
            "modes: power modulation, mask scan, mask lines, and multi-pass."
        ),
        "images": images,
    }

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


def _plot_raster_result(gray, ops, title):
    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    axes[0].imshow(gray, cmap="gray", vmin=0, vmax=255, origin="lower")
    axes[0].set_title("Input image")
    axes[0].set_aspect("equal")

    cmap = plt.get_cmap("hot")
    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if abs(pos[0] - ep[0]) > 1e-6 or abs(pos[1] - ep[1]) > 1e-6:
                axes[1].plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.5,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            sd = ops.scanline_data(i)
            n = len(sd)
            if n > 0:
                xs = np.linspace(pos[0], ep[0], n)
                ys = np.linspace(pos[1], ep[1], n)
                power_arr = np.frombuffer(sd, dtype=np.uint8).astype(
                    np.float64
                )
                power_norm = power_arr / 255.0
                colors = cmap(power_norm)
                colors[:, 3] = np.clip(power_norm * 2, 0.15, 1.0)
                axes[1].scatter(xs, ys, c=colors, s=4, marker="s")
            pos = ep

    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)
    axes[1].set_title(title)

    fig.tight_layout()
    return fig


def generate_examples(output_dir):
    images = []
    img_size = 64
    gray = make_pattern(img_size, img_size, "Circle")
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
        fig = _plot_raster_result(gray, ops, f"Raster: {name}")
        fname = f"rasterization-{name.lower().replace(' ', '-')}.png"
        fig.savefig(output_dir / fname, dpi=150)
        plt.close(fig)
        images.append(
            {"path": fname, "caption": f"Rasterization mode: {name}"}
        )

    return {
        "title": "Rasterization",
        "description": (
            "Rasterize images into laser control sequences using various "
            "modes: power modulation, mask scan, mask lines, and multi-pass."
        ),
        "images": images,
    }

"""Generate rasterize-scanlines example images."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.image import rasterize_scanlines
from raygeo.ops.raster import ScanMode, rasterize_power_modulation
from raygeo.ops.types import CommandType
from tools.plot import make_pattern


def generate_examples(output_dir):
    images = []
    img_size = 64
    ppm = 10.0

    gray = make_pattern(img_size, img_size, "Radial")
    alpha = np.full((img_size, img_size), 255, dtype=np.uint8)

    ops = rasterize_power_modulation(
        gray,
        alpha,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval_mm=0.1,
        sample_interval_mm=0.05,
        min_power=0.0,
        max_power=1.0,
        angle=0,
        scan_mode=ScanMode.Segmented,
    )

    rasterized = rasterize_scanlines(
        ops, img_size, img_size, (ppm, ppm), (0.0, 0.0)
    )

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    max_mm = img_size / ppm

    ops.preload_state()
    power_cmap = plt.get_cmap("plasma")
    pos = (0.0, 0.0, 0.0)
    for i in range(len(ops)):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            pos = ep
            continue
        if ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            n = max(1, len(ops.scanline_data(i)))
            xs = np.linspace(pos[0], ep[0], n)
            ys = np.linspace(pos[1], ep[1], n)
            pwr = np.frombuffer(ops.scanline_data(i), dtype=np.uint8).astype(
                np.float64
            )
            pn = pwr / 255.0
            for j in range(n - 1):
                axes[0].plot(
                    xs[j : j + 2],
                    ys[j : j + 2],
                    color=power_cmap(pn[j] if j < len(pn) else 0),
                    linewidth=1.2,
                )
            pos = ep

    axes[0].set_xlim(0, max_mm)
    axes[0].set_ylim(0, max_mm)
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)
    axes[0].set_title("Input: Ops scanlines (power-colored)")

    axes[1].imshow(
        rasterized,
        cmap="plasma",
        vmin=0,
        vmax=255,
        origin="lower",
        extent=[0, max_mm, 0, max_mm],
    )
    axes[1].set_title("Output: rasterized power-map buffer")
    axes[1].set_xlim(0, max_mm)
    axes[1].set_ylim(0, max_mm)
    axes[1].set_aspect("equal")

    fig.tight_layout()
    path = output_dir / "rasterize-scanlines.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "rasterize-scanlines.png",
            "caption": "Scanline ops rasterized into a 2D power-map buffer",
        }
    )

    return {
        "title": "Rasterize Scanlines",
        "description": (
            "Rasterize ScanLine commands into a 2D pixel buffer. "
            "Per-pixel power values are max-blended onto the output."
        ),
        "images": images,
    }

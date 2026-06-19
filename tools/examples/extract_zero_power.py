"""Generate extract_zero_power_segments example image."""

__images__ = [
    {
        "stem": "zero-power-segments",
        "caption": "Zero-power segment extraction",
        "doc": "raygeo.ops.raster.md",
        "heading": "extract_zero_power_segments",
    },
]

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import Normalize

from raygeo.ops.raster import extract_zero_power_segments


def generate_examples(output_dir):
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

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

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
    fig.colorbar(sm, ax=ax1, label="Power", shrink=0.7)

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

    fig.tight_layout()
    fname = "zero-power-segments.png"
    fig.savefig(output_dir / fname, dpi=150)
    plt.close(fig)

    return {
        "title": "Zero-Power Segment Extraction",
        "description": (
            "Extract contiguous zero-power segments from scanline power data."
        ),
        "images": [
            {"path": fname, "caption": "Zero-power segment extraction"}
        ],
    }

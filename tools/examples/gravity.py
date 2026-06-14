"""Generate gravity nesting example images."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import to_hex

from raygeo.nest.gravity import apply_gravity


def _make_part(i, size, rng):
    if i % 2 == 0:
        w = size * (0.5 + 0.5 * rng.random())
        h = size * (0.5 + 0.5 * rng.random())
        return [(0, 0), (w, 0), (w, h), (0, h)]
    else:
        leg_w = size * (0.3 + 0.3 * rng.random())
        leg_h = size * (0.3 + 0.3 * rng.random())
        body_w = size * (0.5 + 0.3 * rng.random())
        body_h = size * (0.5 + 0.3 * rng.random())
        return [
            (0, 0),
            (body_w, 0),
            (body_w, leg_h),
            (leg_w, leg_h),
            (leg_w, body_h),
            (0, body_h),
        ]


def generate_examples(output_dir):
    images = []

    size = 30
    sheet_w, sheet_h = 180, 140
    spacing = 2.0

    rng = np.random.default_rng(42)
    n_parts = 8

    parts = [_make_part(i, size, rng) for i in range(n_parts)]

    cols = 4
    placed_groups = []
    for i, poly in enumerate(parts):
        bx = min(p[0] for p in poly)
        by = min(p[1] for p in poly)
        col = i % cols
        row = i // cols
        ox = col * (size * 1.5) + 10 + rng.uniform(0, size * 0.3)
        oy = row * (size * 1.5) + 10 + rng.uniform(0, size * 0.3)
        shifted = [(p[0] - bx + ox, p[1] - by + oy) for p in poly]
        placed_groups.append([shifted])

    sheet_poly = [
        (0.0, 0.0),
        (sheet_w, 0.0),
        (sheet_w, sheet_h),
        (0.0, sheet_h),
    ]

    adjustments = apply_gravity(placed_groups, sheet_poly, spacing)

    cmap = plt.get_cmap("tab10")

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))

    ax1.plot(
        [p[0] for p in sheet_poly] + [sheet_poly[0][0]],
        [p[1] for p in sheet_poly] + [sheet_poly[0][1]],
        color="black",
        linewidth=2,
        label="Sheet",
    )
    for pi, polys in enumerate(placed_groups):
        for poly in polys:
            px = [p[0] for p in poly] + [poly[0][0]]
            py = [p[1] for p in poly] + [poly[0][1]]
            color = to_hex(cmap(pi % 10))
            ax1.fill(px, py, alpha=0.25, color=color)
            ax1.plot(px, py, color=color, linewidth=1.5)
    ax1.set_aspect("equal")
    ax1.set_title("Before gravity (loose placement)", fontsize=14)
    ax1.grid(True, alpha=0.3)

    ax2.plot(
        [p[0] for p in sheet_poly] + [sheet_poly[0][0]],
        [p[1] for p in sheet_poly] + [sheet_poly[0][1]],
        color="black",
        linewidth=2,
        label="Sheet",
    )
    for pi, (polys, adj) in enumerate(zip(placed_groups, adjustments)):
        for poly in polys:
            shifted = [(p[0] + adj[0], p[1] + adj[1]) for p in poly]
            px = [p[0] for p in shifted] + [shifted[0][0]]
            py = [p[1] for p in shifted] + [shifted[0][1]]
            color = to_hex(cmap(pi % 10))
            ax2.fill(px, py, alpha=0.25, color=color)
            ax2.plot(px, py, color=color, linewidth=1.5)
    ax2.set_aspect("equal")
    ax2.set_title("After gravity (tightened)", fontsize=14)
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    path = output_dir / "gravity.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "gravity.png",
            "caption": "Gravity tightening: before vs after",
        }
    )

    return {
        "title": "Gravity Nesting",
        "description": (
            "Gravity tightens packing by sliding parts down and left."
        ),
        "images": images,
    }

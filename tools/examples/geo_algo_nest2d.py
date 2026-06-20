"""Generate nesting example images."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import to_hex

from raygeo.geo.algo.nest2d.placement import place_parts
from raygeo.geo.shape.polygon import get_polygon_convex_hull


def generate_overview():
    size = 30
    sheet_w, sheet_h = 200, 200
    spacing = 2.0
    rng = np.random.default_rng(42)

    def _make_part(i):
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

    n_parts = 10
    part_polys = [[_make_part(i)] for i in range(n_parts)]
    part_hulls = [
        [get_polygon_convex_hull(part_polys[i][0])] for i in range(n_parts)
    ]
    sheet_poly = [
        (0.0, 0.0),
        (sheet_w, 0.0),
        (sheet_w, sheet_h),
        (0.0, sheet_h),
    ]
    sheet_offsets = [(0.0, 0.0)]
    rotations = [0.0] * n_parts
    fh = [False] * n_parts
    fv = [False] * n_parts

    result = place_parts(
        part_polys,
        part_hulls,
        [sheet_poly],
        sheet_offsets,
        rotations,
        fh,
        fv,
        spacing=spacing,
    )

    fig, ax = plt.subplots(figsize=(10, 8))
    ax.plot(
        [p[0] for p in sheet_poly] + [sheet_poly[0][0]],
        [p[1] for p in sheet_poly] + [sheet_poly[0][1]],
        color="black",
        linewidth=2,
        label="Sheet",
    )

    cmap = plt.get_cmap("tab10")
    if result:
        for pi, pl in enumerate(result[0]["placements"]):
            for poly in pl["polygons"]:
                px = [p[0] for p in poly] + [poly[0][0]]
                py = [p[1] for p in poly] + [poly[0][1]]
                color = to_hex(cmap(pi % 10))
                ax.fill(px, py, alpha=0.25, color=color)
                ax.plot(px, py, color=color, linewidth=1.5)

    ax.set_aspect("equal")
    ax.set_xlim(-spacing * 2, sheet_w + spacing * 2)
    ax.set_ylim(-spacing * 2, sheet_h + spacing * 2)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9, loc="upper right")
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.nest2d.md"]
__images__ = [
    {
        "heading": None,
        "caption": "Part nesting on a sheet",
        "function": generate_overview,
    },
]

"""Generate ClearedArea example images."""

__images__ = [
    {
        "stem": "cleared-area-raster",
        "caption": (
            "ClearedArea tracking a simulated raster toolpath — "
            "cleared fragments shown in blue, remaining area in red"
        ),
        "doc": "raygeo.geo.algo.cleared_area.md",
        "heading": None,
    },
    {
        "stem": "cleared-area-bulk",
        "caption": (
            "ClearedArea with bulk polygon insertion via "
            "``add_cleared_polygons`` — cleared region in blue, "
            "remaining area in red"
        ),
        "doc": "raygeo.geo.algo.cleared_area.md",
        "heading": "add_cleared_polygons",
    },
]

import matplotlib.pyplot as plt

from raygeo.geo.algo.cleared_area import ClearedArea


def generate_examples(output_dir):
    images = []

    # Square pocket 80x80
    boundary = [(5, 5), (85, 5), (85, 85), (5, 85)]

    ca = ClearedArea()
    tool_radius = 3.0
    for i in range(15):
        x = 12.0 + i * 5.0
        if x > 80.0:
            break
        ca.expand([(x, 10), (x, 80)], tool_radius)

    remaining = ca.remaining([boundary])

    fig, ax = plt.subplots(figsize=(7, 7))

    # Draw cleared fragments and remaining area first (behind boundary)
    all_frags = ca.query_window((-10, -10, 100, 100))
    for frag in all_frags:
        fx, fy = zip(
            *([(p[0], p[1]) for p in frag] + [(frag[0][0], frag[0][1])])
        )
        ax.fill(fx, fy, "steelblue", alpha=0.3)
        ax.plot(fx, fy, "steelblue", linewidth=1, alpha=0.6)

    for poly in remaining:
        px, py = zip(
            *([(p[0], p[1]) for p in poly] + [(poly[0][0], poly[0][1])])
        )
        ax.fill(
            px,
            py,
            "tomato",
            alpha=0.4,
            label="Remaining" if poly is remaining[0] else None,
        )
        ax.plot(px, py, "tomato", linewidth=1.5)

    # Boundary drawn last so the black line is visible on top
    bx, by = zip(*(boundary + [boundary[0]]))
    ax.plot(bx, by, "k-", linewidth=2, label="Boundary")

    ax.set_aspect("equal")
    ax.set_xlim(0, 90)
    ax.set_ylim(0, 90)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9)
    ax.set_title(
        f"ClearedArea: {ca.total_area():.0f} mm\u00b2 cleared, "
        f"{len(all_frags)} fragments"
    )

    fig.tight_layout()
    path = output_dir / "cleared-area-raster.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "cleared-area-raster.png",
            "caption": (
                "ClearedArea tracking vertical raster toolpath passes "
                "inside an 80x80 mm pocket. Blue = already cleared, "
                "red = remaining uncleared."
            ),
        }
    )

    # ── add_cleared_polygons: bulk polygon insertion ──────────────────────
    ca2 = ClearedArea()
    # L-shaped pocket
    pocket = [(0, 0), (100, 0), (100, 100), (60, 100), (60, 40), (0, 40)]
    # Bulk cleared region (a large rectangle inside the pocket)
    cleared_bulk = [(10, 10), (90, 10), (90, 90), (10, 90)]
    ca2.add_cleared_polygons([cleared_bulk])
    remaining2 = ca2.remaining([pocket])

    fig2, ax2 = plt.subplots(figsize=(7, 7))

    # Cleared bulk
    cx, cy = zip(*(cleared_bulk + [cleared_bulk[0]]))
    ax2.fill(cx, cy, "steelblue", alpha=0.3, label="Cleared (bulk)")
    ax2.plot(cx, cy, "steelblue", linewidth=1.5)

    # Remaining
    for poly in remaining2:
        px, py = zip(*(poly + [poly[0]]))
        ax2.fill(
            px,
            py,
            "tomato",
            alpha=0.4,
            label="Remaining" if poly is remaining2[0] else None,
        )
        ax2.plot(px, py, "tomato", linewidth=1.5)

    # Boundary
    bx2, by2 = zip(*(pocket + [pocket[0]]))
    ax2.plot(bx2, by2, "k-", linewidth=2, label="Pocket boundary")

    ax2.set_aspect("equal")
    ax2.set_xlim(-5, 105)
    ax2.set_ylim(-5, 105)
    ax2.grid(True, alpha=0.3)
    ax2.legend(fontsize=9)
    ax2.set_title(
        f"ClearedArea.add_cleared_polygons: "
        f"{ca2.total_area():.0f} mm\u00b2 cleared"
    )

    fig2.tight_layout()
    path2 = output_dir / "cleared-area-bulk.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "cleared-area-bulk.png",
            "caption": (
                "Bulk polygon insertion via add_cleared_polygons: "
                "a large rectangle is registered as cleared inside "
                "an L-shaped pocket; remaining material in red."
            ),
        }
    )

    return {
        "title": "ClearedArea",
        "description": (
            "Visualisation of the ClearedArea incremental union of "
            "tool-swept polygons, with spatial-indexed windowed query."
        ),
        "images": images,
    }

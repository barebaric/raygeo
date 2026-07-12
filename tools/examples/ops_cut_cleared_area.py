"""Generate ClearedArea example images."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle

from raygeo.ops.part import StockRegion
from raygeo.ops.part.cleared_area import ClearedArea


def generate_raster():
    """Raster toolpath cleared area."""
    # Square pocket 80x80
    boundary = [(5, 5), (85, 5), (85, 85), (5, 85)]

    region = StockRegion(boundary=boundary)
    ca = ClearedArea()
    tool_radius = 3.0
    for i in range(15):
        x = 12.0 + i * 5.0
        if x > 80.0:
            break
        ca.expand([(x, 10), (x, 80)], tool_radius)

    remaining = ca.remaining(region)

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
        f"ClearedArea: {ca.total_area():.0f} mm² cleared, "
        f"{len(all_frags)} fragments"
    )

    fig.tight_layout()
    return fig


def generate_bulk():
    """Bulk polygon insertion cleared area."""
    # L-shaped pocket
    pocket = [(0, 0), (100, 0), (100, 100), (60, 100), (60, 40), (0, 40)]
    region2 = StockRegion(boundary=pocket)
    ca2 = ClearedArea()
    # Bulk cleared region (a large rectangle inside the pocket)
    cleared_bulk = [(10, 10), (90, 10), (90, 90), (10, 90)]
    ca2.cut([cleared_bulk])
    remaining2 = ca2.remaining(region2)

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
    ax2.set_title(f"ClearedArea.cut: {ca2.total_area():.0f} mm² cleared")

    fig2.tight_layout()
    return fig2


def generate_cut_fast():
    """cut_fast returns only the newly-added portion."""
    ca = ClearedArea()
    # Start with a square cleared area
    initial = [(10, 10), (90, 10), (90, 90), (10, 90)]
    ca.cut([initial])

    # Add a larger square — only the outer ring is new
    larger = [(0, 0), (100, 0), (100, 100), (0, 100)]
    new_ring = ca.cut_fast([larger])

    fig, ax = plt.subplots(figsize=(7, 7))

    # Draw initial cleared region
    ix, iy = zip(*(initial + [initial[0]]))
    ax.fill(ix, iy, "steelblue", alpha=0.3, label="Existing cleared")
    ax.plot(ix, iy, "steelblue", linewidth=1.5)

    # Draw newly-added ring
    for poly in new_ring:
        px, py = zip(*(poly + [poly[0]]))
        ax.fill(
            px,
            py,
            "limegreen",
            alpha=0.4,
            label="New (cut_fast)" if poly is new_ring[0] else None,
        )
        ax.plot(px, py, "limegreen", linewidth=2)

    ax.set_aspect("equal")
    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 105)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9)
    ax.set_title(f"cut_fast: {len(new_ring)} new fragment(s)")

    fig.tight_layout()
    return fig


def generate_frontier():
    """frontier returns a simplified outer boundary."""
    region = StockRegion(boundary=[], islands=[])
    ca = ClearedArea()
    # Two overlapping squares that should be merged
    poly1 = [(10, 10), (60, 10), (60, 60), (10, 60)]
    poly2 = [(40, 40), (90, 40), (90, 90), (40, 90)]
    ca.cut([poly1, poly2])

    f = ca.frontier(region, 0.5)

    fig, ax = plt.subplots(figsize=(7, 7))

    # Draw fragments
    for poly in ca.query_window((-10, -10, 110, 110)):
        px, py = zip(
            *([(p[0], p[1]) for p in poly] + [(poly[0][0], poly[0][1])])
        )
        ax.fill(px, py, "steelblue", alpha=0.2)
        ax.plot(px, py, "steelblue", linewidth=1, alpha=0.5)

    # Draw frontier in bold
    for poly in f:
        fx, fy = zip(*(poly + [poly[0]]))
        ax.plot(
            fx,
            fy,
            "crimson",
            linewidth=3,
            label="Frontier" if poly is f[0] else None,
        )

    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9)
    ax.set_title(
        f"frontier: {len(f)} polygon(s), merged from 2 overlapping fragments"
    )

    fig.tight_layout()
    return fig


def generate_expand_step():
    """show a single expand_step: swept disk enlarges the cleared area."""
    tool_radius = 5.0
    prev = (10, 10)
    nxt = (40, 40)

    # Initial cleared area: a small square the segment does NOT fully cover.
    initial_poly = [(15, 15), (25, 15), (25, 25), (15, 25)]

    ca = ClearedArea()
    ca.cut([initial_poly])

    ca_final = ClearedArea()
    ca_final.cut([initial_poly])
    ca_final.expand_step(prev, nxt, tool_radius)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

    for ax, label, ca_state in [
        (ax1, "Before expand_step", ca),
        (ax2, "After expand_step", ca_final),
    ]:
        for frag in ca_state.query_window((-10, -10, 80, 80)):
            fx = [p[0] for p in frag] + [frag[0][0]]
            fy = [p[1] for p in frag] + [frag[0][1]]
            ax.fill(fx, fy, "steelblue", alpha=0.3)
            ax.plot(fx, fy, "steelblue", linewidth=1.5)

        ax.set_aspect("equal")
        ax.set_xlim(-5, 55)
        ax.set_ylim(-5, 55)
        ax.set_title(label)
        ax.grid(True, alpha=0.3)

    # Draw the segment and tool circles on both panels
    for ax in (ax1, ax2):
        ax.annotate(
            "",
            xy=nxt,
            xytext=prev,
            arrowprops=dict(arrowstyle="->", color="red", lw=2),
        )
        for pt in (prev, nxt):
            c = Circle(
                pt,
                tool_radius,
                fill=False,
                edgecolor="red",
                linewidth=1.5,
                linestyle="--",
            )
            ax.add_patch(c)
        ax.plot(prev[0], prev[1], "ro", markersize=6)
        ax.plot(nxt[0], nxt[1], "ro", markersize=6)

    fig.suptitle(
        "expand_step: swept disk merges into the cleared area",
        fontsize=11,
    )
    fig.tight_layout()
    return fig


def generate_bites():
    """bites across 3 sequential expansion steps."""
    cx, cy = 50.0, 50.0
    step_over = 8.0
    pocket = [(20, 20), (80, 20), (80, 80), (20, 80)]

    def octagon(r):
        return [
            (
                cx + r * math.cos(2 * math.pi * i / 8),
                cy + r * math.sin(2 * math.pi * i / 8),
            )
            for i in range(8)
        ]

    tool_radius = 4.0
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    init = octagon(12.0)
    ca.cut([init])

    # Run 3 sequential bite steps, snapshotting after each
    snapshots = [ca.frontier(region, 0.5)]
    for _ in range(3):
        b = ca.bites(region, step_over, tool_radius, 1.0)
        ca.cut_fast(b)
        snapshots.append(ca.frontier(region, 0.5))

    fig, axes = plt.subplots(1, 3, figsize=(18, 6))
    orange_fill = "#eb8d3b"

    for n in range(1, 4):
        ax = axes[n - 1]

        # Pocket boundary
        bx, by = zip(*(pocket + [pocket[0]]))
        ax.plot(bx, by, "k-", linewidth=2, label="Pocket boundary")

        # Stack all layers up to n — same color, same alpha
        for step in range(n):
            for poly in snapshots[step]:
                px, py = zip(*(poly + [poly[0]]))
                ax.fill(px, py, color=orange_fill, alpha=0.7)
                ax.plot(px, py, color=orange_fill, linewidth=0.5, alpha=0.7)

        ax.set_aspect("equal")
        ax.set_xlim(10, 90)
        ax.set_ylim(10, 90)
        ax.grid(True, alpha=0.3)
        ax.set_title(f"After {n} bite step(s)")
        if n == 1:
            ax.legend(fontsize=8)

    fig.suptitle(
        "bites: 1, 2, and 3 sequential expansions",
        fontsize=13,
    )
    fig.tight_layout()
    return fig


def generate_signed_boundary_distance():
    """Heatmap of signed distance around a cleared rectangle."""
    ca = ClearedArea()
    ca.cut([[(-20, -20), (20, -20), (20, 20), (-20, 20)]])

    xs = np.linspace(-40, 40, 80)
    ys = np.linspace(-40, 40, 80)
    dists = np.zeros((80, 80))
    for i, x in enumerate(xs):
        for j, y in enumerate(ys):
            dists[j, i] = ca.signed_boundary_distance(x, y)

    fig, ax = plt.subplots(figsize=(8, 7))
    im = ax.pcolormesh(xs, ys, dists, shading="auto", cmap="RdYlGn_r")
    cbar = fig.colorbar(im, ax=ax)
    cbar.set_label("Signed distance (mm)")

    for frag in ca.query_window((-50, -50, 50, 50)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.plot(fx, fy, "k-", linewidth=2, label="Boundary")

    ax.set_aspect("equal")
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("Signed Boundary Distance (green = inside, red = outside)")
    fig.tight_layout()
    return fig


def generate_step_batch():
    """Visualise multiple segments batched then committed at once."""
    ca = ClearedArea()
    ca.cut([[(15, 15), (25, 15), (25, 25), (15, 25)]])
    ca.begin_batch()

    # Queue three segments.
    segments = [
        ((10, 10), (20, 30)),
        ((20, 30), (35, 35)),
        ((35, 35), (45, 25)),
    ]
    r = 4.0
    for prev, nxt in segments:
        ca.expand_batched(prev, nxt, r)
    ca.commit_batch()

    fig, ax = plt.subplots(figsize=(8, 7))

    for frag in ca.query_window((-10, -10, 70, 70)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.3)
        ax.plot(fx, fy, "steelblue", linewidth=1.5)

    # Draw the initial seed as a dashed outline.
    ax.plot(
        [15, 25, 25, 15, 15],
        [15, 15, 25, 25, 15],
        "k--",
        linewidth=1.5,
        alpha=0.5,
        label="Initial seed",
    )

    for prev, nxt in segments:
        ax.annotate(
            "",
            xy=nxt,
            xytext=prev,
            arrowprops=dict(arrowstyle="->", color="red", lw=2),
        )
        c = Circle(
            nxt, r, fill=False, edgecolor="red", linewidth=1, linestyle=":"
        )
        ax.add_patch(c)

    ax.set_aspect("equal")
    ax.set_xlim(0, 60)
    ax.set_ylim(0, 50)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("step_batch: 3 Segments Queued → Single Union")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_expand():
    """Sweep a multi-segment path with a disk radius."""
    path = [(10, 10), (20, 30), (40, 25), (55, 40)]
    r = 4.0

    ca = ClearedArea()
    ca.cut([[(15, 15), (25, 15), (25, 25), (15, 25)]])
    ca.expand(path, r)

    fig, ax = plt.subplots(figsize=(8, 7))

    for frag in ca.query_window((-10, -10, 80, 80)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.3)
        ax.plot(fx, fy, "steelblue", linewidth=1.5)

    # Draw the path and disks.
    for i in range(len(path) - 1):
        p0, p1 = path[i], path[i + 1]
        ax.annotate(
            "",
            xy=p1,
            xytext=p0,
            arrowprops=dict(arrowstyle="->", color="red", lw=2),
        )
    for pt in path:
        c = Circle(
            pt, r, fill=False, edgecolor="red", linewidth=1, linestyle=":"
        )
        ax.add_patch(c)

    ax.plot([p[0] for p in path], [p[1] for p in path], "ro", markersize=4)

    ax.set_aspect("equal")
    ax.set_xlim(0, 70)
    ax.set_ylim(0, 60)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("expand: Sweep Disk Along a Polyline Path")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_remaining():
    """Show the remaining area after subtracting cleared fragments."""
    pocket = [(0, 0), (100, 0), (100, 100), (0, 100)]
    cleared = [(20, 20), (80, 20), (80, 80), (20, 80)]

    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    ca.cut([cleared])
    remaining = ca.remaining(region)

    fig, ax = plt.subplots(figsize=(8, 7))

    bx = [p[0] for p in pocket] + [pocket[0][0]]
    by = [p[1] for p in pocket] + [pocket[0][1]]
    ax.plot(bx, by, "k-", linewidth=2, label="Boundary")

    # Fill the outer boundary (remaining area) in blue.
    for i, poly in enumerate(remaining):
        px = [p[0] for p in poly] + [poly[0][0]]
        py = [p[1] for p in poly] + [poly[0][1]]
        if i == 0:
            ax.fill(px, py, "steelblue", alpha=0.25, label="Remaining")
        ax.plot(px, py, "steelblue", linewidth=1.5)

    # Draw subtracted (cleared) on top — opaque red with hatch
    # so it fully covers the blue fill underneath.
    subtracted_frags = ca.query_window((-10, -10, 110, 110))
    for idx, frag in enumerate(subtracted_frags):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(
            fx,
            fy,
            facecolor="white",
            zorder=3,
        )
        ax.fill(
            fx,
            fy,
            facecolor="tomato",
            edgecolor="tomato",
            hatch="///",
            alpha=0.3,
            linewidth=0.5,
            zorder=4,
            label="Subtracted" if idx == 0 else "",
        )

    ax.set_aspect("equal")
    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 105)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("remaining: Uncut Area After Subtraction")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_query_window():
    """Show fragments returned by query_window within a bbox."""
    ca = ClearedArea()
    ca.cut(
        [
            [(10, 10), (40, 10), (40, 40), (10, 40)],
            [(60, 60), (90, 60), (90, 90), (60, 90)],
        ]
    )
    bbox = (0, 0, 50, 50)
    result = ca.query_window(bbox)

    fig, ax = plt.subplots(figsize=(8, 7))

    # Draw all fragments.
    for frag in ca.query_window((-10, -10, 110, 110)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.15)
        ax.plot(fx, fy, "steelblue", linewidth=1, alpha=0.4)

    # Draw the query bounding box.
    bx = [bbox[0], bbox[2], bbox[2], bbox[0], bbox[0]]
    by = [bbox[1], bbox[1], bbox[3], bbox[3], bbox[1]]
    ax.plot(bx, by, "g-", linewidth=2, label="Query bbox")

    # Highlight the returned fragments.
    for frag in result:
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        first_qr = frag is result[0]
        ax.fill(
            fx,
            fy,
            "tomato",
            alpha=0.4,
            label="Query result" if first_qr else "",
        )
        ax.plot(fx, fy, "tomato", linewidth=2)

    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("query_window: Fragments Inside a Bounding Box")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_local_vs_global():
    """Side-by-side: Global vs Local strategy produce identical clearing."""

    def _spiral_segments(n):
        segs = []
        for i in range(n):
            a = i * 0.5
            r = 5.0 + a * 0.3
            prev = (100 + r * math.cos(a), 100 + r * math.sin(a))
            next = (
                100 + (r + 1.0) * math.cos(a + 0.5),
                100 + (r + 1.0) * math.sin(a + 0.5),
            )
            segs.append((prev, next, 3.0))
        return segs

    segs = _spiral_segments(500)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 7))

    cag = ClearedArea()
    for prev, nxt, r in segs:
        cag.expand_step(prev, nxt, r)
    vg = sum(len(p) for p in cag.fragments())
    for poly in cag.fragments():
        xs = [p[0] for p in poly]
        ys = [p[1] for p in poly]
        ax1.fill(xs, ys, alpha=0.4, fc="#1f77b4", ec="#1f77b4", lw=0.3)
    ax1.set_title(f"Global — {len(cag.fragments())} frags, {vg} verts")
    ax1.set_aspect("equal")

    cal = ClearedArea()
    for prev, nxt, r in segs:
        cal.begin_batch()
        cal.expand_batched(prev, nxt, r)
        cal.commit_batch_local()
    vl = sum(len(p) for p in cal.fragments())
    for poly in cal.fragments():
        xs = [p[0] for p in poly]
        ys = [p[1] for p in poly]
        ax2.fill(xs, ys, alpha=0.4, fc="#ff7f0e", ec="#ff7f0e", lw=0.3)
    ax2.set_title(f"Local — {len(cal.fragments())} frags, {vl} verts")
    ax2.set_aspect("equal")

    fig.suptitle("Global vs Local strategy — identical area", fontsize=13)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.part.cleared_area.md"]
__images__ = [
    {
        "heading": None,
        "caption": (
            "ClearedArea tracking a simulated raster toolpath — "
            "cleared fragments in blue, remaining red"
        ),
        "function": generate_raster,
    },
    {
        "heading": "cut",
        "caption": (
            "ClearedArea with bulk polygon insertion via "
            "``cut`` — cleared region in blue, "
            "remaining area in red"
        ),
        "function": generate_bulk,
    },
    {
        "heading": "cut_fast",
        "caption": (
            "``cut_fast`` adds polygons to the cleared area, "
            "returning only the newly-covered region (green)."
        ),
        "function": generate_cut_fast,
    },
    {
        "heading": "frontier",
        "caption": (
            "``frontier`` returns the outer boundary of the cleared area "
            "after merging overlapping fragments."
        ),
        "function": generate_frontier,
    },
    {
        "heading": "signed_boundary_distance",
        "caption": (
            "Signed boundary distance around a cleared square: "
            "green = inside cleared, red = outside."
        ),
        "function": generate_signed_boundary_distance,
    },
    {
        "heading": "expand",
        "caption": (
            "``expand``: sweeping a disk along a multi-segment "
            "path enlarges the cleared area."
        ),
        "function": generate_expand,
    },
    {
        "heading": "remaining",
        "caption": (
            "``remaining`` subtracts cleared fragments from the "
            "boundary, returning the uncut region (red)."
        ),
        "function": generate_remaining,
    },
    {
        "heading": "query_window",
        "caption": (
            "``query_window`` returns only the cleared fragments "
            "whose bbox overlaps the query (green)."
        ),
        "function": generate_query_window,
    },
    {
        "heading": "expand_step",
        "caption": (
            "``expand_step`` sweeps a disk from *prev* to *next*, "
            "enlarging the cleared area vs initial state."
        ),
        "function": generate_expand_step,
    },
    {
        "heading": "begin_batch",
        "caption": (
            "Three segments via ``begin_batch`` / ``expand_batched``, "
            "unioned in a single ``commit_batch``."
        ),
        "function": generate_step_batch,
    },
    {
        "heading": "bites",
        "caption": (
            "``bites`` computes the expansible material reachable "
            "by expanding the frontier by ``step_over``."
        ),
        "function": generate_bites,
    },
    {
        "heading": "commit_batch_local",
        "caption": "Global vs local commit — only overlapping frags updated.",
        "function": generate_local_vs_global,
    },
]

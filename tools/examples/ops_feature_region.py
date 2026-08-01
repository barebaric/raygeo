"""Visualisation for ops/feature/region — wide-region detection.

The primary plot loads the ``barebaric`` text-as-path SVG (the exact
geometry used for the multi-face adaptive-clearing testcases), builds a
multi-face ``Part`` via ``Part.from_geometry_multi_face``, and
marks every region that ``find_regions`` detects on each face.

The SVG is loaded from the test assets at
``tests/svg/barebaric-text-as-path-joined.svg`` (the path is taken from
the ``RAYGEO_SVG`` environment variable when set).  When the file is not
found the generator returns ``None`` so the docs build stays green; a
synthetic H-shape pocket is provided as a second, self-contained image.
"""

import os
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Circle as CirclePatch
from matplotlib.patches import Polygon as PolygonPatch

from raygeo.geo import Geometry, Matrix
from raygeo.ops.feature import region as _region
from raygeo.ops.feature.narrow import analyze_pocket
from raygeo.ops.part import Part
from raygeo.svg import svg_string_to_geometries

find_regions = _region.find_regions

COLORS = ["#4C72B0", "#DD8452", "#55A868", "#C44E52", "#8172B3", "#937860"]

# Passage classes reported by ``analyze_pocket`` — the connector regions
# that separate wide areas.  Same palette as ops_feature_narrow.
CLASS_COLORS = {
    "narrow": "darkorange",
    "slot": "mediumblue",
    "unreachable": "crimson",
}

# Repo root: tools/examples/ops_feature_region.py -> ../../../
_REPO_ROOT = Path(__file__).resolve().parent.parent.parent


def _default_svg_path():
    return _REPO_ROOT / "tests" / "svg" / "barebaric-text-as-path-joined.svg"


def _svg_path():
    return Path(os.environ.get("RAYGEO_SVG", _default_svg_path()))


def _load_svg_part(size_mm=(408.080685314315, 88.80888415102355)):
    """
    Build the multi-face ``Part`` from the SVG via
    ``from_geometry_multi_face``.

    The SVG is parsed to geometries, normalized to the unit square
    (content bounds → 0..1), then scaled by *size_mm* — the same
    sequence used by ``Part.from_geometry_multi_face``.
    """
    path = _svg_path()
    if not path.exists():
        return None
    geos = svg_string_to_geometries(path.read_text(), 1.0, 1.0)
    if not geos:
        return None
    geo = Geometry()
    for g in geos:
        geo.extend(g)
    if geo.is_empty():
        return None
    x0, y0, x1, y1 = geo.rect()
    w, h = x1 - x0, y1 - y0
    if w <= 0 or h <= 0:
        return None
    geo.transform(Matrix.translation(-x0, -y0))
    geo.transform(Matrix.scale(1.0 / w, 1.0 / h))
    # SVG is Y-down; flip to Y-up (flip_matrix = T(0,1) @ S(1,-1)).
    geo.transform(Matrix.scale(1.0, -1.0))
    geo.transform(Matrix.translation(0.0, 1.0))
    geo.transform(Matrix.scale(size_mm[0], size_mm[1]))
    return Part.from_geometry_multi_face(geometry=geo, size_mm=size_mm)


def _plot_passages(ax, passages):
    """Draw the classified narrow/slot/unreachable passages."""
    for poly, cls, _min_w, _entry in passages:
        color = CLASS_COLORS.get(cls, "gray")
        xs = [p[0] for p in poly] + [poly[0][0]]
        ys = [p[1] for p in poly] + [poly[0][1]]
        ax.fill(xs, ys, color=color, alpha=0.35, zorder=2)
        ax.plot(xs, ys, "-", color=color, linewidth=1.2, zorder=2)


def _plot_face(ax, boundary, islands, regions, tool_radius, index=0):
    """Plot one face: boundary, islands, and each region marked.

    ``index`` offsets the region numbers so numbers stay consecutive
    across multiple faces overlaid on the same axes.  Islands are drawn
    on top of the region fills (white interior, gray edge) so they appear
    as genuine holes rather than being painted over.
    """
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "-", color="k", linewidth=1.5, zorder=9)

    for i, (poly, _area, entry_pt, r_max) in enumerate(regions):
        color = COLORS[(index + i) % len(COLORS)]
        verts = list(poly)
        ax.add_patch(
            PolygonPatch(
                verts,
                facecolor=color,
                edgecolor=color,
                alpha=0.4,
                linewidth=1.5,
                label=f"region {index + i + 1}" if i == 0 else None,
                zorder=3,
            )
        )
        cx, cy = entry_pt
        ax.plot(cx, cy, "o", color=color, ms=5, zorder=5)
        ax.add_patch(
            CirclePatch(
                (cx, cy),
                r_max,
                fill=False,
                edgecolor=color,
                linestyle="--",
                linewidth=1.3,
                alpha=0.8,
                zorder=4,
            )
        )
        ax.annotate(
            str(index + i + 1),
            (cx, cy),
            textcoords="offset points",
            xytext=(6, 6),
            fontsize=9,
            fontweight="bold",
            color=color,
            zorder=6,
        )

    # Islands drawn last so they punch clean holes through the fills.
    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(ix, iy, color="white", zorder=7)
        ax.plot(ix, iy, color="dimgray", linewidth=1.2, zorder=8)


def generate_svg_regions():
    """The exact ``barebaric`` text SVG with every region marked.

    All faces are overlaid at their true positions in one figure — even
    faces that yield no regions (e.g. the ``i`` dot) are shown with their
    boundary and islands so nothing is lost.  Every wide sub-region
    detected by ``find_regions`` is filled with a distinct color with its
    entry point and largest inscribed circle; islands punch through as
    white holes.
    """
    tool_radius = 3.0
    tolerance = 1.0
    part = _load_svg_part()
    if part is None:
        return None

    fig, ax = plt.subplots(figsize=(12, 4))

    region_index = 0
    for fid in sorted(part.face_ids):
        f = part.face(fid)
        if f is None:
            continue
        bnd = f.stock_region.boundary
        isls = f.stock_region.islands
        regions = find_regions(bnd, isls, tool_radius, tolerance)
        passages = analyze_pocket(bnd, isls, tool_radius, tolerance)
        _plot_passages(ax, passages)
        # Faces with no regions are still genuine pockets (e.g. the "i"
        # dot): render them as a normal pocket with just a boundary.
        if not regions:
            bx = [p[0] for p in bnd] + [bnd[0][0]]
            by = [p[1] for p in bnd] + [bnd[0][1]]
            ax.fill(bx, by, color="white", zorder=1)
            _plot_face(ax, bnd, isls, [], tool_radius, region_index)
            continue
        _plot_face(ax, bnd, isls, regions, tool_radius, region_index)
        region_index += len(regions)

    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.25)

    legend_items = [
        Line2D([0], [0], color="k", lw=1.5, label="pocket boundary"),
        Line2D(
            [0],
            [0],
            color="dimgray",
            lw=1.2,
            label="island",
        ),
        Line2D([0], [0], color=COLORS[0], lw=1.5, label="wide region"),
        Line2D(
            [0],
            [0],
            color=COLORS[0],
            lw=1.3,
            linestyle="--",
            alpha=0.8,
            label="largest inscribed circle",
        ),
        Line2D(
            [0],
            [0],
            color=CLASS_COLORS["narrow"],
            lw=1.2,
            label="narrow passage",
        ),
        Line2D(
            [0],
            [0],
            color=CLASS_COLORS["slot"],
            lw=1.2,
            label="slot passage",
        ),
        Line2D(
            [0],
            [0],
            color=CLASS_COLORS["unreachable"],
            lw=1.2,
            label="unreachable passage",
        ),
    ]
    fig.legend(
        handles=legend_items,
        loc="lower center",
        ncol=4,
        fontsize=9,
        frameon=False,
        bbox_to_anchor=(0.5, -0.02),
    )
    fig.suptitle(
        "find_regions on the barebaric text-as-path SVG "
        f"(tool_radius={tool_radius}, tolerance={tolerance})",
        fontsize=12,
    )
    fig.tight_layout()
    return fig


def generate_find_regions():
    """Triple-dumbbell H-shape: 3 wide lobes connected by 2 narrow corridors.

    The geometry has three wide areas:
    - Left lobe: 20x30 rectangle
    - Centre lobe: 20x30 rectangle
    - Right lobe: 20x30 rectangle
    Connected by corridors 25x5 mm (narrow for tool_radius=3).
    """
    tool_radius = 3.0
    tolerance = 0.5

    # H-shape: three lobes connected by narrow corridors
    # Left lobe: x=-20..0, y=-15..15
    # Centre lobe: x=25..45, y=-15..15
    # Right lobe: x=70..90, y=-15..15
    # Corridor 1: x=0..25, y=-2.5..2.5
    # Corridor 2: x=45..70, y=-2.5..2.5
    pocket = [
        (-20.0, -15.0),
        (0.0, -15.0),
        (0.0, -2.5),
        (25.0, -2.5),
        (25.0, -15.0),
        (45.0, -15.0),
        (45.0, -2.5),
        (70.0, -2.5),
        (70.0, -15.0),
        (90.0, -15.0),
        (90.0, 15.0),
        (70.0, 15.0),
        (70.0, 2.5),
        (45.0, 2.5),
        (45.0, 15.0),
        (25.0, 15.0),
        (25.0, 2.5),
        (0.0, 2.5),
        (0.0, 15.0),
        (-20.0, 15.0),
    ]

    regions = find_regions(
        boundary=pocket,
        islands=None,
        tool_radius=tool_radius,
        tolerance=tolerance,
    )

    fig, ax = plt.subplots(figsize=(10, 5))

    # Draw pocket boundary
    _plot_polygon(ax, pocket, "none", "k", lw=2.0, alpha=0.0)
    xs = [p[0] for p in pocket] + [pocket[0][0]]
    ys = [p[1] for p in pocket] + [pocket[0][1]]
    ax.plot(xs, ys, "-", color="k", linewidth=2.0, label="pocket boundary")

    # Color palette for regions
    colors = COLORS

    for i, r in enumerate(regions):
        poly, _area, entry_pt, r_max = r
        color = colors[i % len(colors)]
        # Fill the wide region
        poly_verts = list(poly)
        patch = PolygonPatch(
            poly_verts,
            facecolor=color,
            edgecolor=color,
            alpha=0.35,
            linewidth=1.5,
            label=f"region {i + 1}" if i < 3 else None,
            zorder=3,
        )
        ax.add_patch(patch)
        # Draw entry point
        cx, cy = entry_pt
        ax.plot(cx, cy, "o", color=color, ms=6, zorder=5)
        ax.add_patch(
            CirclePatch(
                (cx, cy),
                r_max,
                fill=False,
                edgecolor=color,
                linestyle="--",
                linewidth=1.5,
                alpha=0.8,
                zorder=4,
            )
        )

    # Also show the narrow passage outlines
    # (They separate the wide regions)
    corridor1 = [
        (0.0, -2.5),
        (25.0, -2.5),
        (25.0, 2.5),
        (0.0, 2.5),
    ]
    corridor2 = [
        (45.0, -2.5),
        (70.0, -2.5),
        (70.0, 2.5),
        (45.0, 2.5),
    ]
    for corr in [corridor1, corridor2]:
        cx = [p[0] for p in corr] + [corr[0][0]]
        cy = [p[1] for p in corr] + [corr[0][1]]
        ax.fill(
            cx,
            cy,
            facecolor="gray",
            alpha=0.3,
            edgecolor="gray",
            linewidth=1.0,
            linestyle="--",
            label="narrow passage" if corr is corridor1 else None,
        )

    ax.set_title("Disconnected wide regions in an H-shape pocket", fontsize=12)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.25)
    ax.set_xlim(-25, 95)
    ax.set_ylim(-20, 20)

    legend_items = [
        Line2D([0], [0], color="k", lw=2.0, label="pocket boundary"),
        Line2D(
            [0],
            [0],
            color="gray",
            lw=1.0,
            linestyle="--",
            label="narrow passage",
        ),
        Line2D([0], [0], color=colors[0], lw=1.5, label="wide region"),
        Line2D(
            [0],
            [0],
            color=colors[0],
            lw=1.5,
            linestyle="--",
            alpha=0.8,
            label="largest inscribed circle",
        ),
    ]
    fig.legend(
        handles=legend_items,
        loc="lower center",
        ncol=4,
        fontsize=9,
        frameon=False,
        bbox_to_anchor=(0.5, -0.02),
    )

    fig.tight_layout()
    return fig


def _plot_polygon(ax, poly, face, edge, **kwargs):
    xs = [p[0] for p in poly] + [poly[0][0]]
    ys = [p[1] for p in poly] + [poly[0][1]]
    ax.fill(xs, ys, facecolor=face, alpha=kwargs.get("alpha", 0.25))
    ax.plot(xs, ys, "-", color=edge, linewidth=kwargs.get("lw", 1.5))


__docs_target__ = ["raygeo.ops.feature.region.md"]
__images__ = [
    {
        "heading": "find_regions",
        "caption": (
            "The barebaric text SVG: each face's wide sub-regions,"
            " entry points, and largest inscribed circles"
        ),
        "function": generate_svg_regions,
    },
    {
        "heading": "find_regions",
        "caption": (
            "H-shape pocket: wide regions colored, entry points marked,"
            " narrow corridors shaded gray"
        ),
        "function": generate_find_regions,
    },
]

if __name__ == "__main__":
    fig = generate_svg_regions()
    if fig is not None:
        fig.savefig(
            "/tmp/ops_feature_region_svg.png", dpi=150, bbox_inches="tight"
        )
        print("Saved /tmp/ops_feature_region_svg.png")
    else:
        print("SVG not found; generating synthetic H-shape only")
    fig = generate_find_regions()
    fig.savefig("/tmp/ops_feature_region.png", dpi=150, bbox_inches="tight")
    print("Saved /tmp/ops_feature_region.png")

"""Generate visualisations of wavefront motion assembly."""

import pathlib

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import Normalize

from raygeo.geo.shape.polygon import (
    get_polygon_signed_area,
    is_point_inside_polygon,
)
from raygeo.ops.assembly.entry import adaptive_entry
from raygeo.ops.assembly.wavefront import adaptive_wavefronts
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.svg import svg_string_to_geometries


def _ops_to_points(ops):
    """Extract (x, y, z, is_travel) for every moving command in *ops*."""
    pts = []
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            ep = ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return pts


def _plot_wavefront_2d(ops, boundary, islands, title):
    """Plot wavefront contours from Ops, coloured by iteration."""
    fig, ax = plt.subplots(figsize=(7, 6))
    subpaths = ops.split_into_subpaths()
    n_wf = len(subpaths)
    cmap = plt.colormaps["plasma"]
    for i, sub in enumerate(subpaths):
        color = cmap(i / max(n_wf - 1, 1))
        pts_list = _ops_to_points(sub)
        seg_x, seg_y = [], []
        last_x = last_y = None
        last_was_travel = False
        for x, y, z, is_travel in pts_list:
            if seg_x and is_travel and not last_was_travel:
                if len(seg_x) >= 2:
                    ax.plot(
                        seg_x, seg_y, color=color, linewidth=0.6, alpha=0.7
                    )
                seg_x, seg_y = [], []
            if not is_travel:
                # Include the preceding travel endpoint so every edge
                # of the ring is drawn.
                if not seg_x and last_was_travel and last_x is not None:
                    seg_x.append(last_x)
                    seg_y.append(last_y)
                seg_x.append(x)
                seg_y.append(y)
            last_x, last_y = x, y
            last_was_travel = is_travel
        if len(seg_x) >= 2:
            ax.plot(seg_x, seg_y, color=color, linewidth=0.6, alpha=0.7)

    bnd = np.array(list(boundary) + [boundary[0]])
    ax.plot(bnd[:, 0], bnd[:, 1], "k-", linewidth=2, label="Boundary")
    if islands:
        for isl in islands:
            isl_arr = np.array(list(isl) + [isl[0]])
            ax.fill(
                isl_arr[:, 0],
                isl_arr[:, 1],
                facecolor="#ccc",
                edgecolor="#999",
                linewidth=1.5,
                label="Island" if isl is islands[0] else None,
            )
    ax.set_aspect("equal")
    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)

    sm = plt.cm.ScalarMappable(cmap=cmap, norm=Normalize(0, n_wf - 1))
    sm.set_array([])
    fig.colorbar(sm, ax=ax, label="Iteration")

    fig.tight_layout()
    return fig


def generate_wavefront_rect():
    """Wavefront rectangular."""
    wf_boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    _, wf_cp = adaptive_entry(
        pocket_boundary=wf_boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    wf_ca = ClearedArea(boundary=wf_boundary, initial=wf_cp)
    wf_ops = adaptive_wavefronts(
        wf_ca,
        wf_boundary,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    return _plot_wavefront_2d(
        wf_ops,
        wf_boundary,
        None,
        "Adaptive Wavefronts — Rectangular Pocket",
    )


def generate_wavefront_multi():
    """Wavefront multi-island."""
    mi_boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    mi_islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [(70, 40), (90, 40), (90, 60), (70, 60)],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    _, mi_cp = adaptive_entry(
        pocket_boundary=mi_boundary,
        islands=mi_islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    mi_ca = ClearedArea(
        boundary=mi_boundary, islands=mi_islands, initial=mi_cp
    )
    mi_ops = adaptive_wavefronts(
        mi_ca,
        mi_boundary,
        islands=mi_islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    return _plot_wavefront_2d(
        mi_ops,
        mi_boundary,
        mi_islands,
        "Adaptive Wavefronts — Multi-Island Pocket",
    )


def generate_wavefront_yshape():
    """Wavefront Y-shape."""
    yshape = [
        (45, 0),
        (75, 0),
        (75, 40),
        (110, 110),
        (80, 110),
        (60, 55),
        (40, 110),
        (10, 110),
        (45, 40),
    ]
    _, ys_cp = adaptive_entry(
        pocket_boundary=yshape,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ys_ca = ClearedArea(boundary=yshape, initial=ys_cp)
    ys_ops = adaptive_wavefronts(
        ys_ca,
        yshape,
        tool_radius=3.0,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    return _plot_wavefront_2d(
        ys_ops,
        yshape,
        None,
        "Adaptive Wavefronts — Y-Shaped Channel",
    )


def generate_wavefront_svg():
    """Adaptive wavefront filling the letters of a complex SVG logo."""
    svg_path = (
        pathlib.Path(__file__).resolve().parent.parent.parent
        / "tests"
        / "svg"
        / "raygeo.svg"
    )
    svg_str = svg_path.read_text()
    geoms = svg_string_to_geometries(svg_str)

    # Collect all polygon vertices (before any transform)
    all_polys_raw = []
    for g in geoms:
        polys = g.to_polygons(tolerance=0.1)
        all_polys_raw.extend(polys)

    # Flip Y: SVG Y-down → math Y-up
    all_y = [y for p in all_polys_raw for _, y in p]
    y_min, y_max = min(all_y), max(all_y)
    all_polys = []
    for p in all_polys_raw:
        flipped = [(x, y_min + y_max - y) for x, y in p]
        all_polys.append(flipped)

    # Separate outer (CW after flip) and inner (CCW after flip) by signed area
    # After Y-flip, SVG outer (CCW in SVG) becomes CW → negative area
    outer_polys = [p for p in all_polys if get_polygon_signed_area(p) < 0]
    inner_polys = [p for p in all_polys if get_polygon_signed_area(p) >= 0]

    # For each outer contour, find its associated holes
    components = []
    for boundary in outer_polys:
        holes = [
            inner
            for inner in inner_polys
            if is_point_inside_polygon(inner[0], boundary)
        ]
        components.append((boundary, holes))

    # Run wavefront on every letter component independently
    results = []
    max_subpaths = 0
    for boundary, islands in components:
        _, cp = adaptive_entry(
            pocket_boundary=boundary,
            islands=islands,
            tool_radius=1.5,
            step_over=1.0,
            safe_z=2.0,
            target_z=-5.0,
            plunge_pitch=1.0,
        )
        ca = ClearedArea(boundary=boundary, islands=islands, initial=cp)
        ops = adaptive_wavefronts(
            ca,
            boundary,
            islands=islands,
            tool_radius=1.5,
            step_over=0.5,
            z=-5.0,
            area_tolerance=0.2,
        )
        n_sub = len(ops.split_into_subpaths())
        max_subpaths = max(max_subpaths, n_sub)
        results.append((ops, boundary, islands))

    # Plot everything
    fig, ax = plt.subplots(figsize=(12, 6))
    cmap = plt.colormaps["plasma"]

    # Background: fill all letter shapes with holes punched out
    for poly in outer_polys:
        arr = np.array(list(poly) + [poly[0]])
        ax.fill(
            arr[:, 0],
            arr[:, 1],
            facecolor="#e8e8e8",
            edgecolor="none",
            zorder=0,
        )
    for poly in inner_polys:
        arr = np.array(list(poly) + [poly[0]])
        ax.fill(
            arr[:, 0], arr[:, 1], facecolor="white", edgecolor="none", zorder=1
        )

    # Plot all wavefronts with a single consistent colormap
    for ops, _boundary, _islands in results:
        subpaths = ops.split_into_subpaths()
        for i, sub in enumerate(subpaths):
            color = cmap(i / max(max_subpaths - 1, 1))
            pts_list = _ops_to_points(sub)
            seg_x, seg_y = [], []
            last_x = last_y = None
            last_was_travel = False
            for x, y, z, is_travel in pts_list:
                if seg_x and is_travel and not last_was_travel:
                    if len(seg_x) >= 2:
                        ax.plot(
                            seg_x,
                            seg_y,
                            color=color,
                            linewidth=0.9,
                            alpha=0.85,
                        )
                    seg_x, seg_y = [], []
                if not is_travel:
                    if not seg_x and last_was_travel and last_x is not None:
                        seg_x.append(last_x)
                        seg_y.append(last_y)
                    seg_x.append(x)
                    seg_y.append(y)
                last_x, last_y = x, y
                last_was_travel = is_travel
            if len(seg_x) >= 2:
                ax.plot(seg_x, seg_y, color=color, linewidth=0.9, alpha=0.85)

    # Letter outlines (thin, dark)
    for poly in outer_polys:
        arr = np.array(list(poly) + [poly[0]])
        ax.plot(arr[:, 0], arr[:, 1], color="#444", linewidth=0.8, zorder=2)
    for poly in inner_polys:
        arr = np.array(list(poly) + [poly[0]])
        ax.plot(arr[:, 0], arr[:, 1], color="#444", linewidth=0.8, zorder=2)

    ax.set_aspect("equal")
    ax.set_title("Adaptive Wavefronts — Raygeo Logo")
    ax.set_axis_off()

    sm = plt.cm.ScalarMappable(cmap=cmap, norm=Normalize(0, max_subpaths - 1))
    sm.set_array([])
    fig.colorbar(sm, ax=ax, label="Iteration")

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.wavefront.md"]

__images__ = [
    {
        "heading": "adaptive_wavefronts",
        "caption": (
            "Adaptive wavefronts expanding outward from the initial cleared"
            " disk (blue) to fill the pocket boundary (black)"
        ),
        "function": generate_wavefront_rect,
    },
    {
        "heading": "adaptive_wavefronts",
        "caption": (
            "Adaptive wavefronts in a pocket with three islands — contours"
            " wrap around each island as they expand"
        ),
        "function": generate_wavefront_multi,
    },
    {
        "heading": "adaptive_wavefronts",
        "caption": (
            "Adaptive wavefronts in a Y-shaped channel — contours split"
            " and propagate along each branch"
        ),
        "function": generate_wavefront_yshape,
    },
    {
        "heading": "adaptive_wavefronts",
        "caption": (
            "Adaptive wavefronts expanding within a complex shape loaded"
            " from an SVG file — contours adapt to the irregular boundary"
            " and wrap around internal islands"
        ),
        "function": generate_wavefront_svg,
    },
]

"""Pure plotting helpers — no Streamlit dependency."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.collections import LineCollection
from matplotlib.colors import Normalize
from matplotlib.path import Path as MPath
from mpl_toolkits.mplot3d.art3d import Line3DCollection, Poly3DCollection

from raygeo.geo import Arc, Bezier, Line, Move


def plot_geometry(
    axes,
    geom,
    color="steelblue",
    label=None,
    show_points=False,
    linewidth=1.5,
):
    """Plot geometry commands as one polyline per contour.

    Merging each contour into a single :meth:`axes.plot` call avoids
    anti-aliasing seams where segments join.
    """
    contours: list[list[tuple[float, float]]] = []
    vertices: list[tuple[float, float]] = []
    current: list[tuple[float, float]] | None = None
    for cmd in geom.iter_typed_commands():
        end = cmd.end
        if isinstance(cmd, Move):
            current = [(end[0], end[1])]
            contours.append(current)
            vertices.append((end[0], end[1]))
        elif isinstance(cmd, Line):
            assert current is not None
            current.append((end[0], end[1]))
            vertices.append((end[0], end[1]))
        elif isinstance(cmd, Arc):
            assert current is not None
            cx = current[-1][0] + cmd.center_offset[0]
            cy = current[-1][1] + cmd.center_offset[1]
            r = math.sqrt(
                cmd.center_offset[0] ** 2 + cmd.center_offset[1] ** 2
            )
            a_start = math.atan2(current[-1][1] - cy, current[-1][0] - cx)
            a_end = math.atan2(end[1] - cy, end[0] - cx)
            angles = _arc_angles(a_start, a_end, cmd.clockwise)
            for a in angles[1:]:
                current.append((cx + r * math.cos(a), cy + r * math.sin(a)))
            current[-1] = (end[0], end[1])  # ensure exact end point
            vertices.append((end[0], end[1]))
        elif isinstance(cmd, Bezier):
            assert current is not None
            c1 = cmd.control1
            c2 = cmd.control2
            px0, py0 = current[-1]
            ts = np.linspace(0, 1, 64)
            bx = (
                (1 - ts) ** 3 * px0
                + 3 * (1 - ts) ** 2 * ts * c1[0]
                + 3 * (1 - ts) * ts**2 * c2[0]
                + ts**3 * end[0]
            )
            by = (
                (1 - ts) ** 3 * py0
                + 3 * (1 - ts) ** 2 * ts * c1[1]
                + 3 * (1 - ts) * ts**2 * c2[1]
                + ts**3 * end[1]
            )
            for i in range(1, len(ts)):
                current.append((bx[i], by[i]))
            vertices.append((end[0], end[1]))

    for ci, contour in enumerate(contours):
        xs = [p[0] for p in contour]
        ys = [p[1] for p in contour]
        lbl = label if ci == 0 else None
        axes.plot(xs, ys, color=color, linewidth=linewidth, label=lbl)

    if show_points:
        for p in vertices:
            axes.plot(p[0], p[1], "o", color=color, markersize=3)


def _arc_angles(a_start, a_end, clockwise):
    diff = a_end - a_start
    if clockwise:
        if diff >= 0:
            diff -= 2 * math.pi
    else:
        if diff <= 0:
            diff += 2 * math.pi
    n = max(32, int(abs(diff) * 16))
    return [a_start + diff * i / n for i in range(n + 1)]


def auto_limits(geoms):
    xs, ys = [], []
    for g in geoms:
        if g.is_empty():
            continue
        r = g.rect()
        xs += [r[0], r[2]]
        ys += [r[1], r[3]]
    if not xs:
        return -10, 10, -10, 10
    pad = max((max(xs) - min(xs)), (max(ys) - min(ys))) * 0.1 + 1
    return min(xs) - pad, max(xs) + pad, min(ys) - pad, max(ys) + pad


def geometry_to_mpath(geom):
    cmds = geom.iter_typed_commands()
    if not cmds:
        return None

    vertices = []
    codes = []
    prev_end = None

    for cmd in cmds:
        end = (cmd.end[0], cmd.end[1])
        if isinstance(cmd, Move):
            if prev_end is not None and codes and codes[-1] != MPath.CLOSEPOLY:
                vertices.append((0, 0))
                codes.append(MPath.CLOSEPOLY)
            vertices.append(end)
            codes.append(MPath.MOVETO)
            prev_end = end
            continue
        if prev_end is None:
            prev_end = end
            continue
        if isinstance(cmd, Line):
            vertices.append(end)
            codes.append(MPath.LINETO)
            prev_end = end
        elif isinstance(cmd, Arc):
            co = cmd.center_offset
            cx = prev_end[0] + co[0]
            cy = prev_end[1] + co[1]
            r = math.sqrt(co[0] ** 2 + co[1] ** 2)
            a_start = math.atan2(prev_end[1] - cy, prev_end[0] - cx)
            a_end = math.atan2(end[1] - cy, end[0] - cx)
            angles = _arc_angles(a_start, a_end, cmd.clockwise)
            for a in angles[1:]:
                vertices.append((cx + r * math.cos(a), cy + r * math.sin(a)))
                codes.append(MPath.LINETO)
            prev_end = end
        elif isinstance(cmd, Bezier):
            c1, c2 = cmd.control1, cmd.control2
            p0 = prev_end
            for t in np.linspace(1 / 20, 1, 20):
                u = 1 - t
                bx = (
                    u**3 * p0[0]
                    + 3 * u**2 * t * c1[0]
                    + 3 * u * t**2 * c2[0]
                    + t**3 * end[0]
                )
                by = (
                    u**3 * p0[1]
                    + 3 * u**2 * t * c1[1]
                    + 3 * u * t**2 * c2[1]
                    + t**3 * end[1]
                )
                vertices.append((bx, by))
                codes.append(MPath.LINETO)
            prev_end = end

    if prev_end is not None and codes and codes[-1] != MPath.CLOSEPOLY:
        if geom.is_closed():
            vertices.append((0, 0))
            codes.append(MPath.CLOSEPOLY)

    return MPath(vertices, codes)


def rasterize_geometries_to_mask(geometries, width, height):
    if not geometries:
        return np.zeros((height, width), dtype=bool)

    xs, ys = [], []
    for g in geometries:
        if g.is_empty():
            continue
        r = g.rect()
        xs += [r[0], r[2]]
        ys += [r[1], r[3]]
    if not xs:
        return np.zeros((height, width), dtype=bool)

    xmin, xmax = min(xs), max(xs)
    ymin, ymax = min(ys), max(ys)
    cw, ch = xmax - xmin, ymax - ymin
    if cw < 1e-6:
        cw = 1.0
    if ch < 1e-6:
        ch = 1.0

    pad = 5
    scale = min((width - 2 * pad) / cw, (height - 2 * pad) / ch)

    offset_x = (width - cw * scale) * 0.5
    offset_y = (height - ch * scale) * 0.5

    mask = np.zeros((height, width), dtype=bool)

    for geom in geometries:
        if geom.is_empty():
            continue
        mpath = geometry_to_mpath(geom)
        if mpath is None:
            continue

        yy, xx = np.mgrid[0:height, 0:width]
        px = (xx - offset_x) / scale + xmin
        py = (yy - offset_y) / scale + ymin
        points = np.column_stack((px.ravel(), py.ravel()))

        inside = mpath.contains_points(points)
        mask |= inside.reshape(height, width)

    return mask


def plot_polygon(ax, pts, color, label, linewidth=1.5):
    if not pts:
        return
    xs = [p[0] for p in pts] + [pts[0][0]]
    ys = [p[1] for p in pts] + [pts[0][1]]
    ax.plot(xs, ys, color=color, linewidth=linewidth, label=label)


def make_pattern(w, h, pattern):
    x = np.arange(w, dtype=np.float64)
    y = np.arange(h, dtype=np.float64)
    xx, yy = np.meshgrid(x, y)

    if pattern == "Gradient":
        arr = ((xx / w) * 255).astype(np.uint8)
    elif pattern == "Checkered":
        block = 16
        checker = ((xx // block) + (yy // block)) % 2 == 0
        arr = np.where(checker, 255, 0).astype(np.uint8)
    elif pattern == "Circle":
        cx, cy = w / 2, h / 2
        dist = np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2)
        r = min(w, h) / 2 * 0.8
        arr = np.where(dist < r, 255, 0).astype(np.uint8)
    elif pattern == "Radial":
        cx, cy = w / 2, h / 2
        dist = np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2)
        r = min(w, h) / 2 * 0.8
        arr = np.where(dist < r, ((1 - dist / r) * 255), 0).astype(np.uint8)
    else:
        rng = np.random.default_rng(42)
        arr = rng.integers(0, 256, (h, w), dtype=np.uint8)

    return arr


def fill_rounded_rect(img, pt1, pt2, r):
    x1, y1 = pt1
    x2, y2 = pt2
    h, w = img.shape
    img[max(0, y1 + r) : min(h, y2 - r), max(0, x1) : min(w, x2)] = True
    img[max(0, y1) : min(h, y2), max(0, x1 + r) : min(w, x2 - r)] = True
    for cy, cx in [
        (y1 + r, x1 + r),
        (y1 + r, x2 - r),
        (y2 - r, x1 + r),
        (y2 - r, x2 - r),
    ]:
        yy, xx = np.ogrid[-r : r + 1, -r : r + 1]
        mask = xx**2 + yy**2 <= r**2
        ys = slice(max(0, cy - r), min(h, cy + r + 1))
        xs = slice(max(0, cx - r), min(w, cx + r + 1))
        img[ys, xs][
            mask[
                : min(h, cy + r + 1) - max(0, cy - r),
                : min(w, cx + r + 1) - max(0, cx - r),
            ]
        ] = True


def plot_ops_2d(
    ax,
    ops,
    *,
    total_edges=None,
    boundary=None,
    islands=None,
    mark_cut_start=False,
    mark_start=True,
    mark_end=True,
):
    """Plot *ops* as a 2-D turbo-coloured toolpath with travel lines.

    Parameters
    ----------
    ax : matplotlib Axes
    ops : Ops
    total_edges : int, optional
        Total expected cut-edge count across all passes.  When given the
        turbo gradient is normalised to this count so partial results use
        only a proportional slice of the spectrum.
    boundary : sequence of (x, y), optional
        Pocket boundary polygon drawn as a black outline underlay.
    islands : list of sequences of (x, y), optional
        Island polygons drawn as light-gray filled underlay.
    mark_cut_start : bool, default False
        If True, mark the start of each cutting segment with a small red dot.
    mark_start : bool, default True
        If True, mark the very first cutting point with a small circle in
        the colour of the lowest gradient value.
    mark_end : bool, default True
        If True, mark the very last cutting point with a small diamond
        (square rotated 45 deg) in the colour of the highest gradient value.

    Returns
    -------
    tuple[tuple[float, float], tuple[float, float]] | None
        ``(start_xy, end_xy)`` of the first and last cutting points,
        or ``None`` when *ops* contains no cutting commands.
    """
    own_handles = []
    if boundary is not None:
        bx = [p[0] for p in boundary] + [boundary[0][0]]
        by = [p[1] for p in boundary] + [boundary[0][1]]
        h = ax.plot(bx, by, "k-", linewidth=1.5, label="Boundary")
        own_handles.extend(h)
    if islands:
        first = True
        for isl in islands:
            ix = [p[0] for p in isl] + [isl[0][0]]
            iy = [p[1] for p in isl] + [isl[0][1]]
            lbl = "Island" if first else "_nolegend_"
            h = ax.fill(ix, iy, color="lightgray", alpha=0.6, label=lbl)
            own_handles.extend(h)
            first = False

    pts = []
    cut_starts = []
    pos = None
    prev_travel = True
    for i in range(ops.len()):
        if ops.is_cutting(i) or ops.is_travel(i):
            ep = ops.endpoint(i)
            if ops.is_cutting(i) and prev_travel and pos is not None:
                pts.append((pos[0], pos[1], False))
                cut_starts.append((pos[0], pos[1]))
            if ops.is_cutting(i) and prev_travel and pos is None:
                cut_starts.append((ep[0], ep[1]))
            pts.append((ep[0], ep[1], ops.is_travel(i)))
            pos = ep
            prev_travel = ops.is_travel(i)
    if not pts:
        return None

    first_cut = None
    last_cut = None
    for x, y, is_travel in pts:
        if not is_travel:
            if first_cut is None:
                first_cut = (x, y)
            last_cut = (x, y)
    start_xy = first_cut if first_cut is not None else (pts[0][0], pts[0][1])
    end_xy = last_cut if last_cut is not None else (pts[-1][0], pts[-1][1])

    segments = []
    cur = []
    for x, y, is_travel in pts:
        if is_travel:
            if len(cur) > 1:
                segments.append(cur)
            cur = []
        else:
            cur.append((x, y))
    if len(cur) > 1:
        segments.append(cur)

    segs = []
    edge_idx = 0
    for seg in segments:
        for j in range(1, len(seg)):
            segs.append([seg[j - 1], seg[j]])
            edge_idx += 1
    total = edge_idx if total_edges is None else total_edges
    if segs:
        colors = plt.cm.turbo([i / max(total, 1) for i in range(len(segs))])
        ax.add_collection(
            LineCollection(segs, colors=colors, linewidth=0.8, alpha=1.0)
        )

    prev = None
    has_travel = False
    for x, y, is_travel in pts:
        if is_travel and prev is not None:
            has_travel = True
            ax.plot(
                [prev[0], x],
                [prev[1], y],
                linestyle="--",
                linewidth=0.5,
                color="dimgray",
                alpha=0.8,
            )
        prev = (x, y)

    if has_travel:
        h = ax.plot(
            [],
            [],
            linestyle="--",
            linewidth=0.5,
            color="dimgray",
            alpha=0.8,
            label="Travel",
        )
        own_handles.extend(h)

    if segs:
        cmap = plt.cm.turbo
        norm = Normalize(vmin=0, vmax=1)
        sm = plt.cm.ScalarMappable(cmap=cmap, norm=norm)
        sm.set_array([])
        fig = ax.get_figure()
        cbar = fig.colorbar(
            sm,
            ax=ax,
            orientation="vertical",
            aspect=30,
            shrink=0.3,
            anchor=(0.1, 0.25),
        )
        cbar.set_label("Cut progress", fontsize=8)

        mid = cmap(0.5)
        h = ax.plot([], [], color=mid, linewidth=0.8, label="Cut path")
        own_handles.extend(h)

        if mark_cut_start and cut_starts:
            cxs = [p[0] for p in cut_starts]
            cys = [p[1] for p in cut_starts]
            h = ax.scatter(
                cxs,
                cys,
                marker="o",
                color="#e31a1c",
                s=12,
                zorder=5,
                label="Cut segment start",
            )
            own_handles.append(h)

        if mark_start and first_cut is not None:
            h = ax.plot(
                first_cut[0],
                first_cut[1],
                marker="o",
                linestyle="None",
                markersize=3,
                color=cmap(0.0),
                zorder=5,
                label="First cut point",
            )
            own_handles.extend(h)
        if mark_end and last_cut is not None:
            h = ax.plot(
                last_cut[0],
                last_cut[1],
                marker="D",
                linestyle="None",
                markersize=2.5,
                color=cmap(1.0),
                zorder=5,
                label="Last cut point",
            )
            own_handles.extend(h)

        all_handles, all_labels = ax.get_legend_handles_labels()
        own_set = set(id(h) for h in own_handles)
        custom = [
            (hand, lbl)
            for hand, lbl in zip(all_handles, all_labels)
            if id(hand) not in own_set
        ]
        ordered_h = [h for h in own_handles] + [h for h, _ in custom]
        own_labels = [h.get_label() for h in own_handles]
        custom_labels = [lbl for _, lbl in custom]
        ordered_l = own_labels + custom_labels

        ax.legend(
            ordered_h,
            ordered_l,
            loc="upper left",
            bbox_to_anchor=(1.0, 1.01),
            bbox_transform=ax.transAxes,
            fontsize=8,
        )
    else:
        all_handles, all_labels = ax.get_legend_handles_labels()
        own_set = set(id(h) for h in own_handles)
        custom = [
            (hand, lbl)
            for hand, lbl in zip(all_handles, all_labels)
            if id(hand) not in own_set
        ]
        ordered_h = [h for h in own_handles] + [h for h, _ in custom]
        own_labels = [h.get_label() for h in own_handles]
        custom_labels = [lbl for _, lbl in custom]
        ordered_l = own_labels + custom_labels

        ax.legend(
            ordered_h,
            ordered_l,
            loc="upper right",
            fontsize=8,
        )

    ax.set_aspect("equal")

    x_min, x_max = ax.get_xlim()
    y_min, y_max = ax.get_ylim()
    half = max(x_max - x_min, y_max - y_min) / 2
    x_mid = (x_min + x_max) / 2
    y_mid = (y_min + y_max) / 2
    ax.set_xlim(x_mid - half, x_mid + half)
    ax.set_ylim(y_mid - half, y_mid + half)

    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")

    return start_xy, end_xy


def plot_ops_3d(
    ax,
    ops,
    *,
    total_edges=None,
    boundary=None,
    islands=None,
    mark_cut_start=False,
    mark_start=False,
    mark_end=False,
):
    """Plot *ops* as a 3-D turbo-coloured toolpath with travel lines.

    Boundary and islands are drawn at the Z of the first cutting command.

    Parameters
    ----------
    ax : matplotlib Axes3D
    ops : Ops
    total_edges : int, optional
    boundary : sequence of (x, y), optional
    islands : list of sequences of (x, y), optional
    mark_cut_start : bool, default False
    mark_start : bool, default True
    mark_end : bool, default True

    Returns
    -------
    tuple[tuple[float, float], tuple[float, float]] | None
        ``(start_xy, end_xy)`` of the first and last cutting points,
        or ``None`` when *ops* contains no cutting commands.
    """
    # Compute the cutting-plane Z from the first cutting command.
    cut_z = None
    for i in range(ops.len()):
        if ops.is_cutting(i):
            ep = ops.endpoint(i)
            cut_z = ep[2]
            break
    if cut_z is None:
        cut_z = 0.0

    if boundary is not None:
        bnd = np.array(list(boundary) + [boundary[0]])
        ax.plot(
            bnd[:, 0],
            bnd[:, 1],
            zs=cut_z,
            zdir="z",
            color="k",
            linewidth=1.5,
            alpha=0.5,
        )
    if islands:
        for isl in islands:
            arr = np.array(list(isl))
            zs = np.full(len(arr), cut_z)
            verts = [np.column_stack([arr[:, 0], arr[:, 1], zs])]
            ax.add_collection3d(
                Poly3DCollection(verts, color="lightgray", alpha=0.3, zorder=1)
            )
            ax.plot(
                arr[:, 0],
                arr[:, 1],
                zs=cut_z,
                zdir="z",
                color="gray",
                linewidth=1,
                alpha=0.4,
            )

    pts = []
    cut_starts = []
    pos = None
    prev_travel = True
    for i in range(ops.len()):
        if ops.is_cutting(i) or ops.is_travel(i):
            ep = ops.endpoint(i)
            if ops.is_cutting(i) and prev_travel and pos is not None:
                pts.append((pos[0], pos[1], pos[2], False))
                cut_starts.append((pos[0], pos[1]))
            if ops.is_cutting(i) and prev_travel and pos is None:
                cut_starts.append((ep[0], ep[1]))
            pts.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
            pos = ep
            prev_travel = ops.is_travel(i)
    if not pts:
        return None

    first_cut = None
    last_cut = None
    for x, y, _z, is_travel in pts:
        if not is_travel:
            if first_cut is None:
                first_cut = (x, y)
            last_cut = (x, y)
    start_xy = first_cut if first_cut is not None else (pts[0][0], pts[0][1])
    end_xy = last_cut if last_cut is not None else (pts[-1][0], pts[-1][1])

    segments = []
    cur = []
    for x, y, _z, is_travel in pts:
        if is_travel:
            if len(cur) > 1:
                segments.append(cur)
            cur = [(x, y, _z)]
        else:
            cur.append((x, y, _z))
    if len(cur) > 1:
        segments.append(cur)

    segs_3d = []
    cum_dists = []
    cum = 0.0
    prev = None
    for seg in segments:
        for p in seg:
            if prev is not None:
                segs_3d.append([prev, p])
                d = math.sqrt(
                    (p[0] - prev[0]) ** 2
                    + (p[1] - prev[1]) ** 2
                    + (p[2] - prev[2]) ** 2
                )
                cum += d
                cum_dists.append(cum)
            prev = p
        prev = None
    total = cum if cum > 0 else 1.0
    if segs_3d:
        lc3d = Line3DCollection(
            segs_3d,
            colors=plt.cm.turbo([d / total for d in cum_dists]),
            linewidth=0.8,
            alpha=1.0,
        )
        ax.add_collection3d(lc3d)

    prev = None
    for x, y, _z, is_travel in pts:
        if is_travel:
            if prev is not None:
                ax.plot(
                    [prev[0], x],
                    [prev[1], y],
                    [prev[2], _z],
                    linestyle="--",
                    linewidth=0.5,
                    color="dimgray",
                    alpha=0.8,
                )
            prev = (x, y, _z)
        else:
            prev = (x, y, _z)

    if segs_3d:
        cmap = plt.cm.turbo

        if mark_cut_start and cut_starts:
            cxs = [p[0] for p in cut_starts]
            cys = [p[1] for p in cut_starts]
            ax.scatter(
                cxs,
                cys,
                marker="o",
                color="#e31a1c",
                s=12,
                zorder=5,
            )

        if mark_start and first_cut is not None:
            ax.plot(
                first_cut[0],
                first_cut[1],
                0,
                marker="o",
                linestyle="None",
                markersize=3,
                color=cmap(0.0),
                zorder=5,
            )
        if mark_end and last_cut is not None:
            ax.plot(
                last_cut[0],
                last_cut[1],
                0,
                marker="D",
                linestyle="None",
                markersize=2.5,
                color=cmap(1.0),
                zorder=5,
            )

    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.view_init(elev=30, azim=-45)

    xl, xr = ax.get_xlim()
    yl, yr = ax.get_ylim()
    zl, zr = ax.get_zlim()
    half = max(xr - xl, yr - yl, zr - zl) * 0.5
    xm = (xl + xr) * 0.5
    ym = (yl + yr) * 0.5
    zm = (zl + zr) * 0.5
    ax.set_xlim(xm - half, xm + half)
    ax.set_ylim(ym - half, ym + half)
    ax.set_zlim(zm - half, zm + half)

    return start_xy, end_xy


def plot_ops(
    ops,
    *,
    total_edges=None,
    boundary=None,
    islands=None,
    mark_cut_start=False,
    mark_start=True,
    mark_end=True,
):
    """2-D + 3-D side-by-side toolpath visualisation.

    Creates a figure with a 3-D view (left) and a 2-D view (right).
    Legend and colorbar appear only on the 2-D subplot.

    Parameters are the same as for :func:`plot_ops_2d` and
    :func:`plot_ops_3d`.

    Returns
    -------
    matplotlib Figure
    """
    fig = plt.figure(figsize=(14, 6))
    ax3d = fig.add_subplot(1, 2, 1, projection="3d")
    ax2d = fig.add_subplot(1, 2, 2)

    plot_ops_3d(
        ax3d,
        ops,
        total_edges=total_edges,
        boundary=boundary,
        islands=islands,
        mark_cut_start=mark_cut_start,
    )

    plot_ops_2d(
        ax2d,
        ops,
        total_edges=total_edges,
        boundary=boundary,
        islands=islands,
        mark_cut_start=mark_cut_start,
        mark_start=mark_start,
        mark_end=mark_end,
    )

    fig.tight_layout()
    fig.subplots_adjust(wspace=0.4)
    return fig

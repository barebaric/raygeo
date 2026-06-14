"""Pure plotting helpers — no Streamlit dependency."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.path import Path as MPath

from raygeo.geo import Arc, Bezier, Line, Move
from raygeo.ops.types import CommandType


def plot_geometry(
    axes,
    geom,
    color="steelblue",
    label=None,
    show_points=False,
    linewidth=1.5,
):
    cmds = geom.iter_typed_commands()
    if not cmds:
        return
    prev_end = None
    seg_label = label
    for cmd in cmds:
        end = cmd.end
        if isinstance(cmd, Move):
            if show_points:
                axes.plot(end[0], end[1], "o", color=color, markersize=3)
            prev_end = end
            continue
        if prev_end is None:
            prev_end = end
            continue
        if isinstance(cmd, Line):
            axes.plot(
                [prev_end[0], end[0]],
                [prev_end[1], end[1]],
                color=color,
                linewidth=linewidth,
                label=seg_label,
            )
            seg_label = None
        elif isinstance(cmd, Arc):
            cx = prev_end[0] + cmd.center_offset[0]
            cy = prev_end[1] + cmd.center_offset[1]
            r = math.sqrt(
                cmd.center_offset[0] ** 2 + cmd.center_offset[1] ** 2
            )
            a_start = math.atan2(prev_end[1] - cy, prev_end[0] - cx)
            a_end = math.atan2(end[1] - cy, end[0] - cx)
            angles = _arc_angles(a_start, a_end, cmd.clockwise)
            ax_pts = [cx + r * math.cos(a) for a in angles]
            ay_pts = [cy + r * math.sin(a) for a in angles]
            axes.plot(
                ax_pts,
                ay_pts,
                color=color,
                linewidth=linewidth,
                label=seg_label,
            )
            seg_label = None
        elif isinstance(cmd, Bezier):
            c1 = cmd.control1
            c2 = cmd.control2
            ts = np.linspace(0, 1, 64)
            bx = (
                (1 - ts) ** 3 * prev_end[0]
                + 3 * (1 - ts) ** 2 * ts * c1[0]
                + 3 * (1 - ts) * ts**2 * c2[0]
                + ts**3 * end[0]
            )
            by = (
                (1 - ts) ** 3 * prev_end[1]
                + 3 * (1 - ts) ** 2 * ts * c1[1]
                + 3 * (1 - ts) * ts**2 * c2[1]
                + ts**3 * end[1]
            )
            axes.plot(
                bx, by, color=color, linewidth=linewidth, label=seg_label
            )
            seg_label = None
        if show_points:
            axes.plot(end[0], end[1], "o", color=color, markersize=3)
        prev_end = end


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


def plot_ops(
    axes,
    ops,
    color="steelblue",
    label=None,
    show_points=False,
    linewidth=1.5,
    show_travel=False,
    show_power=False,
):
    ops.preload_state()
    last_pt = (0.0, 0.0, 0.0)
    seg_label = label
    draw_color = color
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.SET_POWER:
            continue
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if show_travel and last_pt != ep:
                axes.plot(
                    [last_pt[0], ep[0]],
                    [last_pt[1], ep[1]],
                    color="gray",
                    linewidth=0.5,
                    linestyle=":",
                )
            last_pt = ep
            if show_points:
                axes.plot(ep[0], ep[1], "o", color=draw_color, markersize=3)
            continue
        if ct not in (
            CommandType.LINE_TO,
            CommandType.BEZIER_TO,
            CommandType.ARC_TO,
        ):
            continue
        if show_power:
            st = ops.state(i)
            if st is not None and st.power is not None:
                draw_color = plt.get_cmap("RdYlGn")(st.power)
            else:
                draw_color = color
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            axes.plot(
                [last_pt[0], ep[0]],
                [last_pt[1], ep[1]],
                color=draw_color,
                linewidth=linewidth,
                label=seg_label,
            )
            seg_label = None
            last_pt = ep
            if show_points:
                axes.plot(ep[0], ep[1], "o", color=draw_color, markersize=3)
            continue
        if ct == CommandType.BEZIER_TO:
            ep = ops.endpoint(i)
            info = ops.inspect(i)
            c1 = info.control1
            c2 = info.control2
            if c1 and c2:
                ts = np.linspace(0, 1, 64)
                bx = (
                    (1 - ts) ** 3 * last_pt[0]
                    + 3 * (1 - ts) ** 2 * ts * c1[0]
                    + 3 * (1 - ts) * ts**2 * c2[0]
                    + ts**3 * ep[0]
                )
                by = (
                    (1 - ts) ** 3 * last_pt[1]
                    + 3 * (1 - ts) ** 2 * ts * c1[1]
                    + 3 * (1 - ts) * ts**2 * c2[1]
                    + ts**3 * ep[1]
                )
                axes.plot(
                    bx,
                    by,
                    color=draw_color,
                    linewidth=linewidth,
                    label=seg_label,
                )
                seg_label = None
            last_pt = ep
            continue
        if ct == CommandType.ARC_TO:
            ep = ops.endpoint(i)
            info = ops.inspect(i)
            co = info.center_offset
            cw = info.clockwise
            if co:
                cx = last_pt[0] + co[0]
                cy = last_pt[1] + co[1]
                r = math.sqrt(co[0] ** 2 + co[1] ** 2)
                a_start = math.atan2(last_pt[1] - cy, last_pt[0] - cx)
                a_end = math.atan2(ep[1] - cy, ep[0] - cx)
                angles = _arc_angles(a_start, a_end, cw)
                ax_pts = [cx + r * math.cos(a) for a in angles]
                ay_pts = [cy + r * math.sin(a) for a in angles]
                axes.plot(
                    ax_pts,
                    ay_pts,
                    color=draw_color,
                    linewidth=linewidth,
                    label=seg_label,
                )
                seg_label = None
            last_pt = ep
            continue

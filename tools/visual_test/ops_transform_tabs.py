import math

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import streamlit as st

from raygeo.ops import Ops
from raygeo.ops.types import SectionType
from tools.plot import plot_ops_2d


def page_tabs():
    st.header("Tab Operations")

    c1, c2 = st.columns(2)
    with c1:
        shape = st.selectbox(
            "Shape", ["Rectangle", "Circle", "Rounded Rect"], key="tab_shape"
        )
    with c2:
        mode = st.selectbox("Mode", ["Gap", "Power"], key="tab_mode")

    cx, cy = 10, 10
    if shape == "Rectangle":
        w = st.number_input("Width", 2.0, 100.0, 20.0, key="tab_w")
        h = st.number_input("Height", 2.0, 100.0, 20.0, key="tab_h")
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.set_power(1.0)
        ops.move_to(cx - w / 2, cy - h / 2, 0)
        ops.line_to(cx + w / 2, cy - h / 2, 0)
        ops.line_to(cx + w / 2, cy + h / 2, 0)
        ops.line_to(cx - w / 2, cy + h / 2, 0)
        ops.close_path()
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif shape == "Circle":
        r = st.number_input("Radius", 1.0, 50.0, 10.0, key="tab_r")
        n = st.number_input("Segments", 8, 128, 64, key="tab_n")
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.set_power(1.0)
        ops.move_to(cx + r, cy, 0)
        for i in range(1, n + 1):
            a = 2 * math.pi * i / n
            ops.line_to(cx + r * math.cos(a), cy + r * math.sin(a), 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    else:
        w = st.number_input("Width", 2.0, 100.0, 20.0, key="tab_w2")
        h = st.number_input("Height", 2.0, 100.0, 20.0, key="tab_h2")
        d = min(w, h) * 0.2
        k = 0.5522847498
        kd = k * d
        x0, y0 = cx - w / 2, cy - h / 2
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.set_power(1.0)
        ops.move_to(x0 + d, y0, 0)
        ops.line_to(x0 + w - d, y0, 0)
        ops.bezier_to(
            (x0 + w - d + kd, y0, 0),
            (x0 + w, y0 + d - kd, 0),
            (x0 + w, y0 + d, 0),
        )
        ops.line_to(x0 + w, y0 + h - d, 0)
        ops.bezier_to(
            (x0 + w, y0 + h - d + kd, 0),
            (x0 + w - d + kd, y0 + h, 0),
            (x0 + w - d, y0 + h, 0),
        )
        ops.line_to(x0 + d, y0 + h, 0)
        ops.bezier_to(
            (x0 + d - kd, y0 + h, 0),
            (x0, y0 + h - d + kd, 0),
            (x0, y0 + h - d, 0),
        )
        ops.line_to(x0, y0 + d, 0)
        ops.bezier_to(
            (x0, y0 + d - kd, 0),
            (x0 + d - kd, y0, 0),
            (x0 + d, y0, 0),
        )
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig_ops = ops

    st.subheader("Tab Positions")
    n_tabs = st.number_input("Number of tabs", 0, 10, 2, key="tab_count")
    tab_power = st.slider("Tab power", 0.0, 1.0, 0.1, key="tab_pwr")
    tab_width = st.number_input("Tab width (mm)", 0.1, 20.0, 2.0, key="tab_tw")

    geo = orig_ops.to_geometry()
    segments = geo.segments()
    seg_dists = []
    for seg in segments:
        for j in range(1, len(seg)):
            dx = seg[j][0] - seg[j - 1][0]
            dy = seg[j][1] - seg[j - 1][1]
            seg_dists.append(math.sqrt(dx * dx + dy * dy))
    total_dist = sum(seg_dists)

    clips = []
    if total_dist > 0 and n_tabs > 0:
        flat_pts = [p for seg in segments for p in seg]
        for t in range(n_tabs):
            target = total_dist * (t + 1) / (n_tabs + 1)
            accum = 0.0
            for seg_i, sd in enumerate(seg_dists):
                if accum + sd >= target - 1e-9:
                    frac = (target - accum) / sd if sd > 1e-9 else 0.0
                    px = flat_pts[seg_i][0] + frac * (
                        flat_pts[seg_i + 1][0] - flat_pts[seg_i][0]
                    )
                    py = flat_pts[seg_i][1] + frac * (
                        flat_pts[seg_i + 1][1] - flat_pts[seg_i][1]
                    )
                    clips.append((px, py, tab_width))
                    break
                accum += sd

    result_ops = orig_ops.copy()
    if clips:
        if mode == "Gap":
            result_ops.apply_tab_gaps(clips)
        else:
            result_ops.apply_tab_power(clips, tab_power, 1.0)

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    axes[0].set_title("Original")
    plot_ops_2d(axes[0], orig_ops)
    for cx_, cy_, tw_ in clips:
        axes[0].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
        axes[0].add_patch(
            mpatches.Circle(
                (cx_, cy_),
                tw_ / 2,
                fill=False,
                color="red",
                linestyle="--",
                linewidth=1,
            )
        )
    axes[0].grid(True, alpha=0.3)

    axes[1].set_title(f"After {mode} Tabs")
    plot_ops_2d(axes[1], result_ops)
    for cx_, cy_, tw_ in clips:
        axes[1].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Original commands", orig_ops.len())
    c2.metric("Result commands", result_ops.len())
    c3.metric("Original cut dist", f"{orig_ops.cut_distance():.2f} mm")

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import streamlit as st

from raygeo.ops import Ops
from raygeo.ops.types import CommandType


def page_ops_clip():
    st.header("Ops Clipping")
    st.write("Clip Ops paths to rectangles and polygonal regions.")

    preset = st.selectbox(
        "Shape preset",
        ["Nested squares + triangle", "Overlapping lines", "Cross pattern"],
        key="clip_preset",
    )

    ops = Ops()
    ops.set_power(1.0)

    if preset == "Nested squares + triangle":
        ops.move_to(10, 10, 0)
        ops.line_to(90, 10, 0)
        ops.line_to(90, 90, 0)
        ops.line_to(10, 90, 0)
        ops.line_to(10, 10, 0)
        ops.move_to(30, 30, 0)
        ops.line_to(70, 30, 0)
        ops.line_to(70, 70, 0)
        ops.line_to(30, 70, 0)
        ops.line_to(30, 30, 0)
        ops.move_to(20, 40, 0)
        ops.line_to(80, 40, 0)
        ops.line_to(50, 80, 0)
        ops.line_to(20, 40, 0)
    elif preset == "Overlapping lines":
        ops.move_to(5, 50, 0)
        ops.line_to(95, 50, 0)
        ops.move_to(50, 5, 0)
        ops.line_to(50, 95, 0)
        ops.move_to(10, 10, 0)
        ops.line_to(90, 90, 0)
        ops.move_to(10, 90, 0)
        ops.line_to(90, 10, 0)
    else:
        ops.move_to(10, 50, 0)
        ops.line_to(90, 50, 0)
        ops.move_to(50, 10, 0)
        ops.line_to(50, 90, 0)
        ops.move_to(10, 10, 0)
        ops.line_to(90, 90, 0)
        ops.move_to(90, 10, 0)
        ops.line_to(10, 90, 0)

    c1, c2 = st.columns(2)
    with c1:
        rx1 = st.slider("Clip rect X1", 0.0, 100.0, 25.0, key="clip_rx1")
        ry1 = st.slider("Clip rect Y1", 0.0, 100.0, 25.0, key="clip_ry1")
    with c2:
        rx2 = st.slider("Clip rect X2", 0.0, 100.0, 75.0, key="clip_rx2")
        ry2 = st.slider("Clip rect Y2", 0.0, 100.0, 85.0, key="clip_ry2")

    clip_rect = (rx1, ry1, rx2, ry2)
    clipped = ops.clip_rect(clip_rect)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))

    def _plot(ax, seq, title):
        seq.preload_state()
        pos = (0.0, 0.0, 0.0)
        for i in range(seq.len()):
            ct = seq.command_type(i)
            if ct == CommandType.MOVE_TO:
                pos = seq.endpoint(i)
                continue
            if ct == CommandType.LINE_TO:
                ep = seq.endpoint(i)
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="steelblue",
                    linewidth=2.5,
                    solid_capstyle="round",
                )
                pos = ep
        rect = mpatches.Rectangle(
            (clip_rect[0], clip_rect[1]),
            clip_rect[2] - clip_rect[0],
            clip_rect[3] - clip_rect[1],
            fill=False,
            edgecolor="tomato",
            linewidth=2,
            linestyle="--",
            label="Clip rect",
        )
        ax.add_patch(rect)
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=10)
        ax.set_title(title, fontsize=13)

    _plot(ax1, ops, "Original paths")
    _plot(ax2, clipped, "After clip_rect")

    fig.tight_layout()
    st.pyplot(fig)

    c1, c2 = st.columns(2)
    c1.metric("Original commands", ops.len())
    c2.metric("Clipped commands", clipped.len())

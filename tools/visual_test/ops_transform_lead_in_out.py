import math

import matplotlib.pyplot as plt
import streamlit as st

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def page_lead_in_out():
    st.header("Lead-In / Lead-Out")

    preset = st.selectbox(
        "Preset",
        [
            "Rectangle",
            "Triangle",
            "Diagonal line",
            "Circle (linearized)",
            "Multiple contours",
        ],
        key="lio_preset",
    )

    c1, c2 = st.columns(2)
    with c1:
        lead_in = st.slider("Lead-in (mm)", 0.0, 20.0, 5.0, 0.5, key="lio_in")
    with c2:
        lead_out = st.slider(
            "Lead-out (mm)", 0.0, 20.0, 5.0, 0.5, key="lio_out"
        )

    ops = Ops()
    ops.set_power(1.0)

    if preset == "Rectangle":
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(20, 20, 0)
        ops.line_to(80, 20, 0)
        ops.line_to(80, 80, 0)
        ops.line_to(20, 80, 0)
        ops.line_to(20, 20, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif preset == "Triangle":
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(50, 10, 0)
        ops.line_to(90, 80, 0)
        ops.line_to(10, 80, 0)
        ops.line_to(50, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif preset == "Diagonal line":
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(90, 90, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif preset == "Circle (linearized)":
        n = 64
        r = 35
        cx, cy = 50, 50
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(cx + r, cy, 0)
        for i in range(1, n + 1):
            a = 2 * math.pi * i / n
            ops.line_to(cx + r * math.cos(a), cy + r * math.sin(a), 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif preset == "Multiple contours":
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(40, 10, 0)
        ops.line_to(40, 40, 0)
        ops.line_to(10, 40, 0)
        ops.line_to(10, 10, 0)
        ops.move_to(60, 60, 0)
        ops.line_to(90, 60, 0)
        ops.line_to(90, 90, 0)
        ops.line_to(60, 90, 0)
        ops.line_to(60, 60, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig = ops.copy()
    orig_lines = len(ops.indices_of(CommandType.LINE_TO))

    ops.apply_lead_in_out(lead_in, lead_out)

    result_lines = len(ops.indices_of(CommandType.LINE_TO))

    fig, ax = plt.subplots(figsize=(10, 10))

    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if pos != ep:
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            state = ops.state(i)
            color = (
                "dodgerblue" if state and state.power < 0.01 else "forestgreen"
            )
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=color,
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot(
        [], [], color="forestgreen", linewidth=2.5, label="Cut (power > 0)"
    )
    ax.plot(
        [], [], color="dodgerblue", linewidth=2.5, label="Lead (power = 0)"
    )
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Lines", f"{orig_lines} -> {result_lines}")
    c2.metric("Lead-in", f"{lead_in:.1f} mm")
    c3.metric("Lead-out", f"{lead_out:.1f} mm")

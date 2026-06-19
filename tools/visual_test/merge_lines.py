import matplotlib.pyplot as plt
import streamlit as st

from raygeo.ops import Ops
from raygeo.ops.types import CommandType


def page_merge_lines():
    st.header("Merge Lines")

    preset = st.selectbox(
        "Preset",
        [
            "Near-duplicate lines (tolerance-sensitive)",
            "Identical duplicates",
            "Overlapping collinear",
            "Adjacent rectangles",
            "Triangle shared edge",
            "Custom",
        ],
        key="ml_preset",
    )

    ops = Ops()
    ops.set_power(1.0)

    if preset == "Near-duplicate lines (tolerance-sensitive)":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.move_to(0, 1.5)
        ops.line_to(100, 1.5)
        ops.move_to(0, 5)
        ops.line_to(100, 5)
    elif preset == "Identical duplicates":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.move_to(0, 0)
        ops.line_to(100, 0)
    elif preset == "Overlapping collinear":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.move_to(50, 0)
        ops.line_to(150, 0)
    elif preset == "Adjacent rectangles":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.line_to(100, 100)
        ops.line_to(0, 100)
        ops.line_to(0, 0)
        ops.move_to(100, 0)
        ops.line_to(200, 0)
        ops.line_to(200, 100)
        ops.line_to(100, 100)
        ops.line_to(100, 0)
    elif preset == "Triangle shared edge":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.line_to(50, 100)
        ops.line_to(0, 0)
        ops.move_to(100, 0)
        ops.line_to(0, 0)
        ops.line_to(50, -100)
        ops.line_to(100, 0)
    else:
        pts_text = st.text_area(
            "Segments (one per line: x1,y1 -> x2,y2)",
            "0,0 -> 100,0\n0,0 -> 100,0",
            key="ml_custom",
        )
        for line in pts_text.strip().splitlines():
            parts = line.strip().split("->")
            if len(parts) == 2:
                start = [float(v) for v in parts[0].strip().split(",")]
                end = [float(v) for v in parts[1].strip().split(",")]
                ops.move_to(start[0], start[1])
                ops.line_to(end[0], end[1])

    tol = st.slider("Tolerance", 0.0, 5.0, 1.0, 0.1, key="ml_tol")

    orig = ops.copy()
    orig_lines = len(ops.indices_of(CommandType.LINE_TO))
    orig_moves = len(ops.indices_of(CommandType.MOVE_TO))
    orig_cut = ops.cut_distance()

    ops.merge_overlapping_lines(tol)

    result_lines = len(ops.indices_of(CommandType.LINE_TO))
    result_moves = len(ops.indices_of(CommandType.MOVE_TO))
    result_cut = ops.cut_distance()

    fig, ax = plt.subplots(figsize=(12, 8))

    ax.set_title(
        f"Tolerance={tol:.1f}  |  "
        f"Original: {orig_lines} lines, {orig_moves} moves, "
        f"cut={orig_cut:.1f}  ->  "
        f"Merged: {result_lines} lines, {result_moves} moves, "
        f"cut={result_cut:.1f}"
    )

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
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
                label="Merged result" if i == 0 else None,
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.set_aspect("equal")
    xl = ax.get_xlim()
    yl = ax.get_ylim()
    pad = max(xl[1] - xl[0], yl[1] - yl[0]) * 0.05 + 5
    ax.set_xlim(xl[0] - pad, xl[1] + pad)
    ax.set_ylim(yl[0] - pad, yl[1] + pad)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

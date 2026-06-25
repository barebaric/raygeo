import matplotlib.pyplot as plt
import streamlit as st

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def page_ops_optimize_travel():
    st.header("Travel Optimization")
    st.write("Optimize the travel (non-cutting) path by reordering segments.")

    preset = st.selectbox(
        "Preset",
        ["Multiple rectangles", "Triangle + squares", "Scattered segments"],
        key="opt_preset",
    )

    ops = Ops()
    ops.set_power(1.0)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")

    if preset == "Multiple rectangles":
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.line_to(30, 30, 0)
        ops.line_to(10, 30, 0)
        ops.line_to(10, 10, 0)
        ops.move_to(50, 50, 0)
        ops.line_to(70, 50, 0)
        ops.line_to(70, 70, 0)
        ops.line_to(50, 70, 0)
        ops.line_to(50, 50, 0)
        ops.move_to(10, 60, 0)
        ops.line_to(30, 60, 0)
        ops.line_to(30, 80, 0)
        ops.line_to(10, 80, 0)
        ops.line_to(10, 60, 0)
        ops.move_to(60, 10, 0)
        ops.line_to(80, 10, 0)
        ops.line_to(80, 30, 0)
        ops.line_to(60, 30, 0)
        ops.line_to(60, 10, 0)
    elif preset == "Triangle + squares":
        ops.move_to(5, 75, 0)
        ops.line_to(20, 75, 0)
        ops.line_to(20, 90, 0)
        ops.line_to(5, 90, 0)
        ops.line_to(5, 75, 0)
        ops.move_to(80, 5, 0)
        ops.line_to(95, 5, 0)
        ops.line_to(95, 20, 0)
        ops.line_to(80, 20, 0)
        ops.line_to(80, 5, 0)
        ops.move_to(50, 40, 0)
        ops.line_to(70, 80, 0)
        ops.line_to(30, 80, 0)
        ops.line_to(50, 40, 0)
    else:
        ops.move_to(10, 10, 0)
        ops.line_to(40, 10, 0)
        ops.move_to(60, 70, 0)
        ops.line_to(80, 50, 0)
        ops.move_to(30, 80, 0)
        ops.line_to(50, 80, 0)
        ops.line_to(50, 60, 0)
        ops.move_to(70, 20, 0)
        ops.line_to(90, 20, 0)
        ops.line_to(90, 40, 0)

    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig = ops.copy()
    ops_noflip = ops.copy()
    ops_flip = ops.copy()
    ops_noflip.optimize_travel(allow_flip=False)
    ops_flip.optimize_travel(allow_flip=True)

    before_travel = orig.distance() - orig.cut_distance()
    travel_noflip = ops_noflip.distance() - ops_noflip.cut_distance()
    travel_flip = ops_flip.distance() - ops_flip.cut_distance()

    fig, (ax1, ax2, ax3) = plt.subplots(1, 3, figsize=(22, 7))

    def _plot(ax, seq, title, travel_d):
        seq.preload_state()
        pos = (0.0, 0.0, 0.0)
        for i in range(seq.len()):
            ct = seq.command_type(i)
            if ct == CommandType.MOVE_TO:
                ep = seq.endpoint(i)
                if pos != ep:
                    ax.annotate(
                        "",
                        xy=(ep[0], ep[1]),
                        xytext=(pos[0], pos[1]),
                        arrowprops=dict(
                            arrowstyle="->",
                            color="gray",
                            lw=1.5,
                            linestyle=":",
                        ),
                    )
                pos = ep
                continue
            if ct == CommandType.LINE_TO:
                ep = seq.endpoint(i)
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="steelblue",
                    linewidth=3,
                    solid_capstyle="round",
                )
                pos = ep
        ax.plot([], [], color="steelblue", linewidth=3, label="Cut")
        ax.plot(
            [], [], color="gray", linewidth=1.5, linestyle=":", label="Travel"
        )
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=10)
        ax.set_title(f"{title}\nTravel: {travel_d:.1f}", fontsize=12)

    _plot(ax1, orig, "Before optimization", before_travel)
    _plot(ax2, ops_noflip, "Optimized (no flip)", travel_noflip)
    _plot(ax3, ops_flip, "Optimized (with flip)", travel_flip)

    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Cut distance", f"{orig.cut_distance():.1f}")
    c2.metric("Travel (no flip)", f"{travel_noflip:.1f}")
    c3.metric("Travel (with flip)", f"{travel_flip:.1f}")

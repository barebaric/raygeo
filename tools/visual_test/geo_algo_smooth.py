import math

import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.algo.smooth import (
    compute_gaussian_kernel,
    smooth_circularly,
    smooth_polyline_3d,
    smooth_sub_segment,
)
from raygeo.geo.shape.polygon3d import resample_polyline_3d


def page_smoothing():
    st.header("Polyline Smoothing")
    st.write("Smooth polylines using Gaussian kernel filtering.")

    tab_basic, tab_advanced, tab_subseg = st.tabs(
        ["Basic", "Advanced", "Sub-Segment"]
    )

    with tab_basic:
        st.subheader("Basic Gaussian Smoothing")
        amount = st.slider(
            "Smoothing amount (kernel size)", 1, 15, 5, key="smooth_amount"
        )
        corner_thresh = st.slider(
            "Corner angle threshold (degrees)",
            0.0,
            180.0,
            45.0,
            key="smooth_corner",
        )
        is_closed = st.checkbox(
            "Close polyline", value=False, key="smooth_closed"
        )

        # Create a simple open polyline with some sharp corners
        pts = [
            (0, 0, 0),
            (4, 0, 0),
            (4, 5, 0),
            (8, 5, 0),
            (8, 0, 0),
            (12, 0, 0),
        ]

        smoothed = smooth_polyline_3d(
            [(x, y, 0) for x, y, z in pts],
            amount,
            math.radians(corner_thresh),
            is_closed,
        )

        fig, ax = plt.subplots(figsize=(10, 6))
        ax.plot(
            [p[0] for p in pts],
            [p[1] for p in pts],
            "o-",
            color="steelblue",
            linewidth=2,
            markersize=6,
            label="Original",
        )
        ax.plot(
            [p[0] for p in smoothed],
            [p[1] for p in smoothed],
            "o-",
            color="tomato",
            linewidth=2,
            markersize=4,
            alpha=0.7,
            label=f"Smoothed (amount={amount}, corner={corner_thresh}°)",
        )
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend()
        ax.set_title("Polyline Smoothing")
        st.pyplot(fig)

    with tab_advanced:
        st.subheader("Circular Smoothing (Closed Loop)")
        amount_circ = st.slider("Kernel size", 1, 15, 7, key="circ_amount")

        # Create a polygon with sharp corners (not circular)
        loop_pts = [
            (0, 0, 0),
            (10, 0, 0),
            (15, 5, 0),
            (10, 10, 0),
            (0, 10, 0),
            (0, 0, 0),
        ]

        kernel, sigma = compute_gaussian_kernel(amount_circ)
        st.info(
            f"Kernel: {kernel[:3]}... (size={len(kernel)}, sigma={sigma:.3f})"
        )

        smoothed_circ = smooth_circularly(loop_pts, kernel)

        fig2, ax2 = plt.subplots(figsize=(10, 6))
        ax2.plot(
            [p[0] for p in loop_pts],
            [p[1] for p in loop_pts],
            "o-",
            color="steelblue",
            linewidth=2,
            markersize=5,
            label="Original",
        )
        ax2.plot(
            [p[0] for p in smoothed_circ],
            [p[1] for p in smoothed_circ],
            "o-",
            color="tomato",
            linewidth=2,
            markersize=4,
            alpha=0.7,
            label=f"Smoothed (kernel size={amount_circ})",
        )
        ax2.set_aspect("equal")
        ax2.grid(True, alpha=0.3)
        ax2.legend()
        ax2.set_title("Circular Smoothing (Closed Loop)")
        st.pyplot(fig2)

    with tab_subseg:
        st.subheader("Sub-Segment Smoothing")
        st.write("Smooth only a portion of a polyline.")

        seg_amount = st.slider("Kernel size", 1, 11, 5, key="subseg_amount")

        # Create a zigzag pattern
        zigzag = [(i, (-1) ** i * 2, 0) for i in range(12)]

        kernel_seg, _ = compute_gaussian_kernel(seg_amount)
        smoothed_sub = smooth_sub_segment(zigzag, kernel_seg)

        fig3, ax3 = plt.subplots(figsize=(10, 6))
        ax3.plot(
            [p[0] for p in zigzag],
            [p[1] for p in zigzag],
            "o-",
            color="steelblue",
            linewidth=2,
            markersize=6,
            label="Original zigzag",
        )
        ax3.plot(
            [p[0] for p in smoothed_sub],
            [p[1] for p in smoothed_sub],
            "o-",
            color="tomato",
            linewidth=2,
            markersize=4,
            alpha=0.7,
            label=f"Smoothed (kernel size={seg_amount})",
        )
        ax3.set_aspect("equal")
        ax3.grid(True, alpha=0.3)
        ax3.legend()
        ax3.set_title("Sub-Segment Smoothing")
        st.pyplot(fig3)

        st.subheader("Resampling")
        max_seg_len = st.slider(
            "Max segment length", 0.1, 5.0, 1.0, key="resample_len"
        )
        resampled = resample_polyline_3d(zigzag, max_seg_len, is_closed=False)

        fig4, ax4 = plt.subplots(figsize=(10, 6))
        ax4.plot(
            [p[0] for p in zigzag],
            [p[1] for p in zigzag],
            "o-",
            color="steelblue",
            linewidth=2,
            markersize=6,
            label=f"Original ({len(zigzag)} points)",
        )
        ax4.plot(
            [p[0] for p in resampled],
            [p[1] for p in resampled],
            "o-",
            color="tomato",
            linewidth=2,
            markersize=4,
            alpha=0.7,
            label=f"Resampled ({len(resampled)} points)",
        )
        ax4.set_aspect("equal")
        ax4.grid(True, alpha=0.3)
        ax4.legend()
        ax4.set_title(f"Resampling (max segment length = {max_seg_len})")
        st.pyplot(fig4)

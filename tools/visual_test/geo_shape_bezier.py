import matplotlib.pyplot as plt
import numpy as np
import streamlit as st

from raygeo.geo.shape.bezier import (
    flatten_bezier,
    get_bezier_point_at,
    split_bezier,
)


def page_bezier_curves():
    st.header("Bezier Curve Operations")
    st.write("Split, evaluate, and flatten cubic bezier curves.")

    tab_split, tab_eval, tab_flatten = st.tabs(
        ["Split", "Point at t", "Flatten"]
    )

    p0, p1, p2, p3 = (0.0, 0.0), (0.0, 10.0), (12.0, 10.0), (12.0, 0.0)

    def eval_bezier(pts, n=100):
        ts = np.linspace(0, 1, n)
        result = []
        for t in ts:
            u = 1 - t
            x = (
                u**3 * pts[0][0]
                + 3 * u**2 * t * pts[1][0]
                + 3 * u * t**2 * pts[2][0]
                + t**3 * pts[3][0]
            )
            y = (
                u**3 * pts[0][1]
                + 3 * u**2 * t * pts[1][1]
                + 3 * u * t**2 * pts[2][1]
                + t**3 * pts[3][1]
            )
            result.append((x, y))
        return result

    def plot_curve(ax, pts, color, lw=3, label=None, ls="-"):
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        ax.plot(xs, ys, color=color, lw=lw, label=label, ls=ls)

    with tab_split:
        t_split = st.slider("Split parameter t", 0.01, 0.99, 0.4, 0.01)
        left, right = split_bezier(p0, p1, p2, p3, t_split)
        split_pt = get_bezier_point_at(p0, p1, p2, p3, t_split)
        curve = eval_bezier((p0, p1, p2, p3))
        left_curve = eval_bezier(left)
        right_curve = eval_bezier(right)

        fig, ax = plt.subplots(figsize=(8, 6))
        plot_curve(ax, curve, "gray", lw=2, ls="--", label="Original")
        plot_curve(ax, left_curve, "tomato", lw=3, label="Left half")
        plot_curve(ax, right_curve, "forestgreen", lw=3, label="Right half")
        ax.plot(
            split_pt[0],
            split_pt[1],
            "*",
            color="gold",
            markersize=15,
            zorder=5,
        )
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=11)
        ax.set_xlim(-1, 14)
        ax.set_ylim(-1, 12)
        st.pyplot(fig)

    with tab_eval:
        t_eval = st.slider("Parameter t", 0.0, 1.0, 0.6, 0.01, key="bez_eval")
        pt = get_bezier_point_at(p0, p1, p2, p3, t_eval)
        cp = [(0.0, 0.0), (0.0, 10.0), (12.0, 10.0), (12.0, 0.0)]
        pt = get_bezier_point_at(cp[0], cp[1], cp[2], cp[3], t_eval)
        curve = eval_bezier(cp)
        fig, ax = plt.subplots(figsize=(8, 6))
        plot_curve(ax, curve, "steelblue", lw=3)
        for cpi, cpv in enumerate(cp):
            ax.plot(cpv[0], cpv[1], "o", color="gray", markersize=6, zorder=4)
            ax.plot(pt[0], pt[1], "o", color="tomato", markersize=12, zorder=5)
            ax.annotate(
                f"t={t_eval}",
                pt,
                xytext=(8, 8),
                textcoords="offset points",
                fontsize=12,
                color="tomato",
                fontweight="bold",
            )
            ax.set_aspect("equal")
            ax.grid(True, alpha=0.3)
            ax.set_xlim(-1, 14)
            ax.set_ylim(-1, 12)
            st.pyplot(fig)

    with tab_flatten:
        c_tol, c_nsub = st.columns(2)
        tolerance = c_tol.slider("Tolerance", 0.1, 10.0, 2.0, 0.1)
        max_sub = c_nsub.number_input("Max subdivisions", 1, 64, 20, 1)
        pts = []
        p0_3d = (p0[0], p0[1], 0.0)
        p1_3d = (p1[0], p1[1], 0.0)
        p2_3d = (p2[0], p2[1], 0.0)
        p3_3d = (p3[0], p3[1], 0.0)
        flatten_bezier(p0_3d, p1_3d, p2_3d, p3_3d, tolerance, max_sub, pts)

        fig, ax = plt.subplots(figsize=(8, 6))
        curve = eval_bezier((p0, p1, p2, p3))
        plot_curve(ax, curve, "steelblue", lw=2, ls="--", label="Original")
        xs_f = [p[0] for p in pts]
        ys_f = [p[1] for p in pts]
        ax.plot(
            xs_f,
            ys_f,
            "o-",
            color="tomato",
            lw=2,
            markersize=4,
            label=f"Flattened ({len(pts)} pts)",
        )
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_xlim(-1, 14)
        ax.set_ylim(-1, 12)
        ax.legend(fontsize=11)
        st.pyplot(fig)

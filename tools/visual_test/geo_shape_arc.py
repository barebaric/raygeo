import math

import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo import Arc, Geometry
from raygeo.geo.shape.arc import linearize_arc
from tools.plot import plot_geometry


def page_arc_linearize():
    st.header("Arc Linearization")
    st.write("Convert arcs into line segments at adjustable resolution.")

    c1, c2 = st.columns(2)
    r = c1.number_input("Arc radius", 1.0, 50.0, 10.0, key="al_r")
    arc_deg = c2.slider("Arc sweep (degrees)", 10, 360, 180, key="al_sweep")

    resolution = st.slider(
        "Linearization resolution", 0.1, 5.0, 1.0, key="al_res"
    )

    geom = Geometry()
    sweep_rad = math.radians(arc_deg)
    end_x = r * math.cos(sweep_rad)
    end_y = r * math.sin(sweep_rad)
    geom.move_to(r, 0, 0)
    geom.arc_to(end_x, end_y, -r, 0, False, 0)

    cmds = geom.iter_typed_commands()
    first_arc = None
    for cmd in cmds:
        if isinstance(cmd, Arc):
            first_arc = cmd
            break

    fig, axes = st.columns(2)
    fig_mpl, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    plot_geometry(ax1, geom, color="steelblue", linewidth=2.5)
    ax1.set_aspect("equal")
    ax1.grid(True, alpha=0.3)
    ax1.set_title(f"Original arc ({arc_deg}°)", fontsize=13)
    margin = r * 0.3
    ax1.set_xlim(-margin, r * 1.2 + margin)
    ax1.set_ylim(-r * 1.2 - margin, r * 1.2 + margin)

    segments = linearize_arc(first_arc, (r, 0.0, 0.0), resolution)
    pts_x, pts_y = [], []
    for (sx, sy, _), (ex, ey, _) in segments:
        ax2.plot([sx, ex], [sy, ey], color="tomato", linewidth=2.5)
        pts_x.extend([sx, ex])
        pts_y.extend([sy, ey])
    ax2.scatter(pts_x, pts_y, color="tomato", s=20, zorder=3)
    ax2.set_aspect("equal")
    ax2.grid(True, alpha=0.3)
    ax2.set_title(f"Linearized ({len(segments)} segments)", fontsize=13)
    ax2.set_xlim(ax1.get_xlim())
    ax2.set_ylim(ax1.get_ylim())

    fig_mpl.tight_layout()
    st.pyplot(fig_mpl)

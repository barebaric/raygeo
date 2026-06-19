"""Visual test playground for raygeo.

Run with: make visual  (or: streamlit run tools/visual_test.py)
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import streamlit as st

from tools.visual_test.adaptive_clearing import page_adaptive_clearing
from tools.visual_test.analysis import page_analysis
from tools.visual_test.arc_linearize import page_arc_linearize
from tools.visual_test.bezier_curves import page_bezier_curves
from tools.visual_test.circle_intersections import page_circle_intersections
from tools.visual_test.concave_hull import page_concave_hull
from tools.visual_test.geometry import page_geometry
from tools.visual_test.gravity import page_gravity
from tools.visual_test.image import page_image
from tools.visual_test.inner_fit_polygon import page_inner_fit_polygon
from tools.visual_test.lead_in_out import page_lead_in_out
from tools.visual_test.line_intersections import page_line_intersections
from tools.visual_test.merge_lines import page_merge_lines
from tools.visual_test.minkowski import page_minkowski
from tools.visual_test.nesting import page_nesting
from tools.visual_test.ops_clip import page_ops_clip
from tools.visual_test.ops_optimize_travel import page_ops_optimize_travel
from tools.visual_test.overscan import page_overscan
from tools.visual_test.polygon3d import page_polygon3d
from tools.visual_test.polygon_boolean import page_polygon_boolean
from tools.visual_test.polygon_offset import page_offset
from tools.visual_test.rasterization import page_rasterization
from tools.visual_test.svg import page_svg
from tools.visual_test.tabs import page_tabs

st.set_page_config(layout="wide", page_title="raygeo visual test")
st.title("raygeo Visual Test")

page = st.sidebar.radio(
    "Page",
    [
        "Adaptive Clearing",
        "Arc Linearization",
        "Bezier Curves",
        "Circle Intersections",
        "Concave Hull",
        "Geometry",
        "Geometry Analysis",
        "Gravity",
        "Image Processing",
        "Inner Fit Polygon",
        "Lead-In/Out",
        "Line Intersections",
        "Merge Lines",
        "Minkowski Sum",
        "Nesting",
        "Ops Clipping",
        "Overscan",
        "Polygon 3D",
        "Polygon Boolean",
        "Polygon Offset",
        "Rasterization",
        "SVG Parsing",
        "Tab Operations",
        "Travel Optimization",
    ],
)

if page == "Adaptive Clearing":
    page_adaptive_clearing()
elif page == "Arc Linearization":
    page_arc_linearize()
elif page == "Bezier Curves":
    page_bezier_curves()
elif page == "Circle Intersections":
    page_circle_intersections()
elif page == "Concave Hull":
    page_concave_hull()
elif page == "Geometry":
    page_geometry()
elif page == "Geometry Analysis":
    page_analysis()
elif page == "Gravity":
    page_gravity()
elif page == "Image Processing":
    page_image()
elif page == "Inner Fit Polygon":
    page_inner_fit_polygon()
elif page == "Lead-In/Out":
    page_lead_in_out()
elif page == "Line Intersections":
    page_line_intersections()
elif page == "Merge Lines":
    page_merge_lines()
elif page == "Minkowski Sum":
    page_minkowski()
elif page == "Nesting":
    page_nesting()
elif page == "Ops Clipping":
    page_ops_clip()
elif page == "Overscan":
    page_overscan()
elif page == "Polygon 3D":
    page_polygon3d()
elif page == "Polygon Boolean":
    page_polygon_boolean()
elif page == "Polygon Offset":
    page_offset()
elif page == "Rasterization":
    page_rasterization()
elif page == "SVG Parsing":
    page_svg()
elif page == "Tab Operations":
    page_tabs()
elif page == "Travel Optimization":
    page_ops_optimize_travel()

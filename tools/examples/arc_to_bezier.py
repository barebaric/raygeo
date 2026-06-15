"""Generate arc-to-bezier conversion example images."""

import matplotlib.lines as mlines
import matplotlib.patches as mpatches
import matplotlib.pyplot as plt

from raygeo.geo import Bezier, Geometry
from tools.plot import plot_geometry


def _plot_bezier_controls(ax, geom, color="crimson", ms=4):
    """Draw handle lines and markers for Bezier control points."""
    prev_end = None
    for cmd in geom.iter_typed_commands():
        if isinstance(cmd, Bezier):
            c1, c2 = cmd.control1, cmd.control2
            end = cmd.end
            # Handle lines: start → c1, c2 → end
            if prev_end is not None:
                ax.plot(
                    [prev_end[0], c1[0]],
                    [prev_end[1], c1[1]],
                    color=color,
                    linewidth=0.8,
                    linestyle=":",
                )
                # Curve point at segment start
                ax.scatter(
                    prev_end[0],
                    prev_end[1],
                    color=color,
                    s=ms**2,
                    marker="o",
                    zorder=5,
                )
            ax.plot(
                [c2[0], end[0]],
                [c2[1], end[1]],
                color=color,
                linewidth=0.8,
                linestyle=":",
            )
            # Control points as crosses
            ax.scatter(
                [c1[0], c2[0]],
                [c1[1], c2[1]],
                color=color,
                s=(ms + 2) ** 2,
                marker="x",
                linewidths=1.2,
                zorder=6,
            )
            prev_end = end
        elif hasattr(cmd, "end"):
            prev_end = cmd.end


def _make_arch():
    """Continuous path: line + arc + arc + line forming an arch."""
    g = Geometry()
    g.move_to(0, -20, 0)
    g.line_to(-15, -20, 0)
    # 90° CCW arc up to the arch peak
    g.arc_to(0, -5, 0, 15, False, 0)
    # 90° CCW arc down the other side
    g.arc_to(15, -20, 15, 0, False, 0)
    g.line_to(0, -20, 0)
    return g


def _make_circle():
    """Full circle from two semicircles."""
    g = Geometry()
    r = 8
    g.move_to(r, 30, 0)
    g.arc_to(-r, 30, -r, 0, True, 0)
    g.arc_to(r, 30, r, 0, True, 0)
    return g


def generate_examples(output_dir):
    images = []

    arch = _make_arch()
    arch_beziers = arch.copy()
    arch_beziers.convert_arcs_to_beziers()

    circle = _make_circle()
    circle_beziers = circle.copy()
    circle_beziers.convert_arcs_to_beziers()

    # ── 2×2 grid: before/after for each shape ─────────────────────────
    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(16, 14))

    for ax in (ax1, ax2):
        ax.set_xlim(-22, 22)
        ax.set_ylim(-27, 5)
    for ax in (ax3, ax4):
        ax.set_xlim(-15, 15)
        ax.set_ylim(20, 40)

    plot_geometry(ax1, arch, color="steelblue", linewidth=2.5)
    ax1.set_aspect("equal")
    ax1.grid(True, alpha=0.3)
    ax1.set_title("Before: arc commands (arch)", fontsize=14)

    plot_geometry(ax2, arch_beziers, color="crimson", linewidth=2.5)
    _plot_bezier_controls(ax2, arch_beziers, color="crimson")
    ax2.set_aspect("equal")
    ax2.grid(True, alpha=0.3)
    ax2.set_title("After: convert_arcs_to_beziers()", fontsize=14)

    plot_geometry(ax3, circle, color="steelblue", linewidth=2.5)
    ax3.set_aspect("equal")
    ax3.grid(True, alpha=0.3)
    ax3.set_title("Before: arc commands (circle)", fontsize=14)

    plot_geometry(ax4, circle_beziers, color="crimson", linewidth=2.5)
    _plot_bezier_controls(ax4, circle_beziers, color="crimson")
    ax4.set_aspect("equal")
    ax4.grid(True, alpha=0.3)
    ax4.set_title("After: convert_arcs_to_beziers() (circle)", fontsize=14)

    legend_elements = [
        mpatches.Patch(color="steelblue", label="Original"),
        mpatches.Patch(color="crimson", label="Bezier conversion"),
        mlines.Line2D(
            [0], [0], color="crimson", linestyle=":", label="Handle line"
        ),
        mlines.Line2D(
            [0],
            [0],
            color="crimson",
            marker="o",
            linestyle="",
            markersize=5,
            label="Curve point",
        ),
        mlines.Line2D(
            [0],
            [0],
            color="crimson",
            marker="x",
            linestyle="",
            markersize=6,
            label="Control point",
        ),
    ]
    fig.legend(
        handles=legend_elements, loc="lower center", ncol=4, fontsize=11
    )
    fig.tight_layout(rect=(0, 0.02, 1, 1))

    path = output_dir / "arc-to-bezier.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "arc-to-bezier.png",
            "caption": (
                "Arc commands converted to Bezier curve approximations"
            ),
        }
    )

    # ── Overlay comparison with controls ────────────────────────────
    fig2, (ax_a, ax_b) = plt.subplots(1, 2, figsize=(16, 7))

    plot_geometry(
        ax_a, arch, color="steelblue", linewidth=3, label="Original arc"
    )
    plot_geometry(
        ax_a,
        arch_beziers,
        color="crimson",
        linewidth=1.5,
        label="Bezier approx",
    )
    _plot_bezier_controls(ax_a, arch_beziers, color="crimson", ms=5)
    ax_a.set_aspect("equal")
    ax_a.grid(True, alpha=0.3)
    ax_a.legend(fontsize=10)
    ax_a.set_title("Arch: arc (blue) + bezier (red) overlay", fontsize=13)
    ax_a.set_xlim(-22, 22)
    ax_a.set_ylim(-27, 5)

    plot_geometry(
        ax_b, circle, color="steelblue", linewidth=3, label="Original arc"
    )
    plot_geometry(
        ax_b,
        circle_beziers,
        color="crimson",
        linewidth=1.5,
        label="Bezier approx",
    )
    _plot_bezier_controls(ax_b, circle_beziers, color="crimson", ms=5)
    ax_b.set_aspect("equal")
    ax_b.grid(True, alpha=0.3)
    ax_b.legend(fontsize=10)
    ax_b.set_title("Circle: arc (blue) + bezier (red) overlay", fontsize=13)
    ax_b.set_xlim(-15, 15)
    ax_b.set_ylim(20, 40)

    fig2.tight_layout()
    path2 = output_dir / "arc-to-bezier-overlay.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "arc-to-bezier-overlay.png",
            "caption": (
                "Overlay showing Bezier curves (with control points) "
                "closely matching the original arcs"
            ),
        }
    )

    return {
        "title": "Arc to Bezier Conversion",
        "description": (
            "Convert all Arc commands in a Geometry to Bezier curve "
            "approximations."
        ),
        "images": images,
    }

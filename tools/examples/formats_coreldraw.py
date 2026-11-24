"""Generate visualisations for raygeo.formats.coreldraw."""

import io
import zipfile
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

from raygeo.formats.coreldraw import detect_format, parse
from tools.plot import plot_geometry

ASSETS = (
    Path(__file__).resolve().parent.parent.parent
    / "tests"
    / "formats"
    / "coreldraw"
    / "assets"
)

PAGE_FRAME = "#c8c8c8"
LAYER_COLORS = ["#4C72B0", "#DD8452", "#55A868", "#C44E52"]
IGNORED = "#a0a0a0"

MAGIC_COLOR = "#dce6f2"
FORM_COLOR = "#f6c99f"
VERSION_COLOR = "#c9e2c4"
PLAIN_COLOR = "#f2f2f2"


def _riff(form: bytes, body: bytes = b"") -> bytes:
    payload = form + body
    return b"RIFF" + len(payload).to_bytes(4, "little") + payload


def _zip(riff: bytes) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("content/riffData.cdr", riff)
    return buffer.getvalue()


def _page(axes, document):
    axes.add_patch(
        Rectangle(
            (0, 0),
            document.page_width,
            document.page_height,
            fill=False,
            edgecolor=PAGE_FRAME,
            linewidth=1.5,
            linestyle="--",
        )
    )
    for index, (name, geometry) in enumerate(document.layers):
        plot_geometry(
            axes,
            geometry,
            color=LAYER_COLORS[index % len(LAYER_COLORS)],
            label=name,
        )
    axes.set_xlim(-0.3, document.page_width + 0.3)
    axes.set_ylim(-0.3, document.page_height + 0.3)
    axes.set_aspect("equal")
    axes.set_axis_off()


def generate_pages():
    """Two drawings read natively: page frame and layer geometry."""
    fig, axes = plt.subplots(1, 2, figsize=(10, 6.6))
    for axes_item, name in zip(axes, ("circle.cdr", "square.cdr")):
        document = parse((ASSETS / name).read_bytes())
        _page(axes_item, document)
        axes_item.set_title(
            f"{name} — {document.page_width:g} × {document.page_height:g} in"
        )
        if len(document.layers) > 1:
            axes_item.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


def generate_detect_format():
    """Recognition of the container and version from the leading bytes."""
    samples = [
        ("CorelDRAW 9", _riff(b"CDR9")),
        ("CorelDRAW 12", _riff(b"CDRC")),
        ("CorelDRAW X6", _riff(b"CDRG")),
        ("ZIP package (X4+)", _zip(_riff(b"CDRG"))),
        ("CMX", _riff(b"CMX1")),
        ("Waldo (1, 2)", b"WL\xb5\x00\x00\x00\x00\x00\x00\x00\x00rest"),
        ("PNG", b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR"),
    ]
    fig, axes = plt.subplots(figsize=(11, 5))
    axes.axis("off")
    axes.set_xlim(-11.5, 22)
    axes.set_ylim(-len(samples) - 0.3, 1.7)
    for row, (label, data) in enumerate(samples):
        detected = detect_format(data)
        axes.text(-0.4, -row + 0.5, label, ha="right", va="center", fontsize=9)
        for column in range(12):
            if column < 4:
                face = MAGIC_COLOR
            elif 8 <= column <= 10:
                face = FORM_COLOR
            elif column == 11:
                face = VERSION_COLOR
            else:
                face = PLAIN_COLOR
            axes.add_patch(
                Rectangle(
                    (column, -row),
                    0.94,
                    0.94,
                    facecolor=face,
                    edgecolor="white",
                )
            )
            byte = data[column] if column < len(data) else 0x20
            axes.text(
                column + 0.47,
                -row + 0.45,
                f"{byte:02X}",
                ha="center",
                va="center",
                fontsize=7,
                family="monospace",
            )
        axes.text(
            12.2,
            -row + 0.5,
            "None" if detected is None else detected.name,
            ha="left",
            va="center",
        )
    axes.text(0, 1.35, "magic", fontsize=8, color="#4C72B0", ha="left")
    axes.text(8, 1.35, "form id", fontsize=8, color="#b55e1e", ha="left")
    axes.text(11.4, 1.35, "version", fontsize=8, color="#3d7a35", ha="left")
    axes.text(13.2, 1.35, "detect_format", fontsize=8, ha="left")
    fig.tight_layout()
    return fig


def generate_parse():
    """The first page is returned, its text converted to outlines."""
    document = parse((ASSETS / "pages.cdr").read_bytes())
    fig, axes = plt.subplots(1, 2, figsize=(10, 6.6))
    for index, axes_item in enumerate(axes):
        axes_item.set_xlim(-0.3, document.page_width + 0.3)
        axes_item.set_ylim(-0.3, document.page_height + 0.3)
        axes_item.set_aspect("equal")
        axes_item.set_axis_off()
        returned = index == 0
        axes_item.add_patch(
            Rectangle(
                (0, 0),
                document.page_width,
                document.page_height,
                fill=False,
                edgecolor=PAGE_FRAME if returned else IGNORED,
                linewidth=1.5,
                linestyle="--" if returned else ":",
            )
        )
        axes_item.set_title(
            "page 1 — glyph outlines" if returned else "page 2 — not read"
        )
    for layer_name, geometry in document.layers:
        plot_geometry(
            axes[0], geometry, color=LAYER_COLORS[0], label=layer_name
        )
    axes[0].legend(loc="upper right", fontsize=8)
    axes[1].text(
        document.page_width / 2,
        document.page_height / 2,
        f"page_count = {document.page_count}",
        ha="center",
        va="center",
        fontsize=12,
        color=IGNORED,
    )
    fig.suptitle(
        f"layers: {len(document.layers)} · "
        f"units_to_mm: {document.units_to_mm:g}"
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.formats.coreldraw.md"]
__images__ = [
    {
        "heading": None,
        "function": generate_pages,
        "caption": (
            "Two drawings parsed natively: page frame, layers and geometry"
        ),
    },
    {
        "heading": "detect_format",
        "function": generate_detect_format,
        "caption": ("Container and version recognized from the leading bytes"),
    },
    {
        "heading": "parse",
        "function": generate_parse,
        "caption": (
            "A two-page document: page one is returned, its text drawn as "
            "glyph outlines"
        ),
    },
]

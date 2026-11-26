"""Shared helpers for the DXF tests."""

from pathlib import Path

ASSETS = Path(__file__).parent / "assets"


def asset(name: str) -> str:
    """The text of a DXF asset file."""
    return (ASSETS / name).read_text()


def document(entities, header=None) -> str:
    """Wrap entity definitions in a minimal DXF document.

    Each entity is a ``(type, layer, [(group_code, value), ...])`` tuple.
    """
    lines = ["0", "SECTION", "2", "HEADER"]
    for name, value in (header or {}).items():
        lines += ["9", name, str(value_code(value)), str(value)]
    lines += ["0", "ENDSEC", "0", "SECTION", "2", "ENTITIES"]
    lines += entity_lines(entities)
    lines += ["0", "ENDSEC", "0", "EOF"]
    return "\n".join(lines) + "\n"


def block_document(blocks, entities) -> str:
    """Wrap block definitions and modelspace entities in a document.

    Each block is ``(name, base, contents)``, where ``base`` is an
    ``(x, y)`` pair and ``contents`` is a list of entities in the same
    form :func:`document` takes.
    """
    lines = ["0", "SECTION", "2", "BLOCKS"]
    for name, (x, y), contents in blocks:
        lines += ["0", "BLOCK", "8", "0", "2", name]
        lines += ["10", str(x), "20", str(y), "30", "0"]
        lines += entity_lines(contents)
        lines += ["0", "ENDBLK"]
    lines += ["0", "ENDSEC", "0", "SECTION", "2", "ENTITIES"]
    lines += entity_lines(entities)
    lines += ["0", "ENDSEC", "0", "EOF"]
    return "\n".join(lines) + "\n"


def entity_lines(entities) -> list[str]:
    """The group-code lines of a list of entity tuples."""
    lines: list[str] = []
    for kind, layer, values in entities:
        lines += ["0", kind, "8", layer]
        for code, value in values:
            lines += [str(code), str(value)]
    return lines


def value_code(value) -> int:
    """The group code a header value needs: 70 for ints, 1 for strings."""
    return 70 if isinstance(value, int) else 1


def line(x1, y1, x2, y2, layer="0", z1=0.0, z2=0.0):
    """A LINE entity."""
    return (
        "LINE",
        layer,
        [
            (10, x1),
            (20, y1),
            (30, z1),
            (11, x2),
            (21, y2),
            (31, z2),
        ],
    )


def insert(block, x, y, layer="0", rotation=0.0, x_scale=1.0, y_scale=1.0):
    """An INSERT entity."""
    return (
        "INSERT",
        layer,
        [
            (2, block),
            (10, x),
            (20, y),
            (30, 0.0),
            (41, x_scale),
            (42, y_scale),
            (43, 1.0),
            (50, rotation),
        ],
    )


def arc(cx, cy, radius, start_deg, end_deg, layer="0"):
    """An ARC entity, counter-clockwise from start to end."""
    return (
        "ARC",
        layer,
        [
            (10, cx),
            (20, cy),
            (30, 0.0),
            (40, radius),
            (50, start_deg),
            (51, end_deg),
        ],
    )


def circle(cx, cy, radius, layer="0"):
    """A CIRCLE entity."""
    return (
        "CIRCLE",
        layer,
        [(10, cx), (20, cy), (30, 0.0), (40, radius)],
    )


def rectangle(x, y, w, h, layer="0"):
    """A closed LWPOLYLINE rectangle."""
    return (
        "LWPOLYLINE",
        layer,
        [
            (90, 4),
            (70, 1),
            (10, x),
            (20, y),
            (10, x + w),
            (20, y),
            (10, x + w),
            (20, y + h),
            (10, x),
            (20, y + h),
        ],
    )


def types(geometry) -> list[str]:
    """The command class names of a geometry."""
    return [type(command).__name__ for command in geometry.data]


def ends(geometry, digits: int = 6) -> list[tuple]:
    """The rounded end points of a geometry's commands."""
    return [
        tuple(round(value, digits) for value in command.end)
        for command in geometry.data
    ]

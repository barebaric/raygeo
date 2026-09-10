"""Generate apply_tangential_knife example images."""

from raygeo.ops import Ops
from raygeo.ops.types import SectionType
from tools.plot import plot_ops


def generate_tangential_knife():
    ops = Ops()
    ops.set_power(1.0)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(10, 20, 0)
    ops.line_to(80, 20, 0)
    ops.line_to(80, 60, 0)
    ops.line_to(30, 60, 0)
    ops.line_to(30, 90, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    ops.apply_tangential_knife(45.0, 1.0, 8.0)
    return plot_ops(ops)


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_tangential_knife",
        "caption": "Tangential knife path with A-axis rotation lifts",
        "function": generate_tangential_knife,
    },
]

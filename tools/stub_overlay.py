"""Post-processes the generated stubs into the honest package
surface: the generated per-class stubs stay as compiled, and the
package-level stub re-exports the classes the reference package
exposes."""

import re
from pathlib import Path

STUBS = Path(__file__).parent.parent / "python" / "raygeo" / "sketch"

CONSTRAINT_CLASSES = [
    ("angle", "AngleConstraint"),
    ("aspect_ratio", "AspectRatioConstraint"),
    ("coincident", "CoincidentConstraint"),
    ("collinear", "CollinearConstraint"),
    ("diameter", "DiameterConstraint"),
    ("distance", "DistanceConstraint"),
    ("drag", "DragConstraint"),
    ("equal_distance", "EqualDistanceConstraint"),
    ("equal_length", "EqualLengthConstraint"),
    ("horizontal", "HorizontalConstraint"),
    ("parallelogram", "ParallelogramConstraint"),
    ("perpendicular", "PerpendicularConstraint"),
    ("point_on_curve", "PointOnCurveConstraint"),
    ("point_on_line", "PointOnLineConstraint"),
    ("radius", "RadiusConstraint"),
    ("rotational", "RotationalConstraint"),
    ("symmetry", "SymmetryConstraint"),
    ("tangent", "TangentConstraint"),
    ("vertical", "VerticalConstraint"),
]


def patch_constraints_package() -> None:
    path = STUBS / "constraints" / "__init__.pyi"
    text = path.read_text()
    exports = ["ANGLE_WEIGHT = builtins.float\n"]
    for module, class_name in CONSTRAINT_CLASSES:
        exports.append(
            f"from .{module} import {class_name}\n"
        )
    block = "\n".join(exports)
    marker = "__all__ = [\n"
    assert marker in text
    text = text.replace(marker, block + "\n" + marker, 1)
    names = [name for _, name in CONSTRAINT_CLASSES]
    text = re.sub(
        r"__all__ = \[\n((?:    \"[a-zA-Z_]+\",\n)+)\]",
        lambda match: "__all__ = [\n"
        + "".join(
            f'    "{name}",\n'
            for name in ["ANGLE_WEIGHT", *names]
        )
        + match.group(1).replace(
            '    "Constraint",\n', '    "Constraint",\n'
        )
        + "]",
        text,
        count=1,
    )
    path.write_text(text)


def main() -> None:
    patch_constraints_package()


if __name__ == "__main__":
    main()

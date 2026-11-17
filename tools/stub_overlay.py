
import re
from pathlib import Path

STUBS = Path(__file__).parent.parent / "python" / "raygeo" / "sketch"

ENTITY_MEMBERS = """
    id: types.EntityID
    type: builtins.str
    construction: builtins.bool
    invisible: builtins.bool
    array_copy: builtins.bool
    constrained: builtins.bool
    fill_color: typing.Optional[tuple[float, float, float, float]]
    def get_point_ids(self) -> builtins.list[types.EntityID]: ...
    def get_endpoint_ids(self) -> builtins.list[types.EntityID]: ...
    def get_junction_point_ids(self) -> builtins.list[types.EntityID]: ...
    def get_helper_ids(self) -> builtins.list[types.EntityID]: ...
    def get_ignorable_unconstrained_points(self) -> builtins.list[types.EntityID]: ...
    def get_rigidly_connected_points(self, point_id: types.EntityID) -> builtins.list[types.EntityID]: ...
    def get_drag_anchor_points(self, point_id: types.EntityID) -> builtins.list[types.EntityID]: ...
    def get_all_frame_point_ids(self, registry: protocols.Registry) -> builtins.list[types.EntityID]: ...
    def clear_helper_references(self) -> None: ...
    def restore_base_flags(self, data: dict) -> None: ...
    def supports_fill(self) -> builtins.bool: ...
    def supports_point_on_curve(self) -> builtins.bool: ...
    def is_radius_entity(self) -> builtins.bool: ...
    def is_closed_loop(self) -> builtins.bool: ...
    def is_edge_entity(self) -> builtins.bool: ...
    def characteristic_length(self, registry: protocols.Registry) -> builtins.float: ...
    def characteristic_length_pairs(self) -> builtins.list[list[types.EntityID]]: ...
    def enclosed_signed_area(self, registry: protocols.Registry) -> typing.Optional[builtins.float]: ...
    def update_constrained_status(self, registry: protocols.Registry, constraints: typing.Any) -> None: ...
    def mirror(self, axis: typing.Any) -> None: ...
    def transform_offsets(self, placement: typing.Any) -> None: ...
    def rewrite_offsets_from(self, template: typing.Any, placement: typing.Any) -> None: ...
    def as_offset_item(self, sketch: typing.Any) -> typing.Any: ...
    def plan_offset(self, registry: protocols.Registry, offset: builtins.float, allocate_id: typing.Any) -> typing.Any: ...
    def geometry_signature(self, registry: protocols.Registry) -> tuple: ...
    def to_dict(self) -> dict: ...
    @staticmethod
    def from_dict(data: dict) -> Entity: ...
    def get_state(self) -> typing.Optional[dict]: ...
    def set_state(self, state: dict) -> None: ...
    def to_geometry(self, registry: protocols.Registry) -> typing.Any: ...
    def to_polyline(self, registry: protocols.Registry, tolerance: builtins.float = 0.1) -> builtins.list[tuple[builtins.float, builtins.float]]: ...
    def to_polylines(self, registry: protocols.Registry, tolerance: builtins.float = 0.1) -> builtins.list[builtins.list[tuple[builtins.float, builtins.float]]]: ...
    def to_polygon_vertices(self, registry: protocols.Registry, forward: builtins.bool = True) -> builtins.list[tuple[builtins.float, builtins.float]]: ...
    def append_to_geometry(self, geo: typing.Any, registry: protocols.Registry, forward: builtins.bool = True) -> None: ...
    def create_fill_geometry(self, registry: protocols.Registry) -> typing.Optional[typing.Any]: ...
    def hit_test(self, mx: builtins.float, my: builtins.float, threshold: builtins.float, registry: protocols.Registry) -> builtins.bool: ...
    def contains_point(self, registry: protocols.Registry, x: builtins.float, y: builtins.float) -> builtins.bool: ...
    def is_contained_by(self, rect: typing.Any, registry: protocols.Registry) -> builtins.bool: ...
    def intersects_rect(self, rect: typing.Any, registry: protocols.Registry) -> builtins.bool: ...
    def signed_distance_to(self, x: builtins.float, y: builtins.float, registry: protocols.Registry) -> builtins.float: ...
    def tangent_at(self, registry: protocols.Registry, point_id: types.EntityID) -> typing.Optional[tuple[builtins.float, builtins.float]]: ...
    def __deepcopy__(self, memo: dict) -> Entity: ...
    def __copy__(self) -> Entity: ...
"""

CONSTRAINT_MEMBERS = """
    value: builtins.float
    expression: typing.Optional[builtins.str]
    status: typing.Any
    user_visible: builtins.bool
    def is_world_anchored(self) -> builtins.bool: ...
    @classmethod
    def can_apply_to(cls, selection: typing.Any, sketch: typing.Any = None) -> builtins.bool: ...
    @classmethod
    def applies_to_entities(cls, entities: typing.Sequence[typing.Any]) -> builtins.bool: ...
    @classmethod
    def get_type_key(cls) -> typing.Optional[builtins.str]: ...
    @staticmethod
    def get_type_name() -> builtins.str: ...
    def targets_segment(self, p1: types.EntityID, p2: types.EntityID, entity_id: typing.Optional[types.EntityID]) -> builtins.bool: ...
    def error(self, reg: protocols.Registry, params: typing.Any = None) -> typing.Any: ...
    def gradient(self, reg: protocols.Registry, params: typing.Any = None) -> dict[types.EntityID, builtins.list[typing.Any]]: ...
    def constrains_radius(self, registry: protocols.Registry, entity_id: types.EntityID) -> builtins.bool: ...
    def to_dict(self) -> dict: ...
    @classmethod
    def from_dict(cls, data: dict) -> Constraint: ...
    def is_hit(self, sx: builtins.float, sy: builtins.float, reg: protocols.Registry, to_screen: typing.Any, element: typing.Any, threshold: builtins.float) -> builtins.bool: ...
    def draw(self, ctx: typing.Any, registry: protocols.Registry, to_screen: typing.Any, is_selected: builtins.bool = False, is_hovered: builtins.bool = False, point_radius: builtins.float = 5.0) -> None: ...
    def get_title(self) -> builtins.str: ...
    def get_subtitle(self, registry: protocols.Registry) -> builtins.str: ...
    def get_edit_subtitle(self) -> builtins.str: ...
    def update_from_context(self, context: dict) -> None: ...
    def depends_on_points(self, point_ids: builtins.set[types.EntityID]) -> builtins.bool: ...
    def depends_on_entities(self, entity_ids: builtins.set[types.EntityID]) -> builtins.bool: ...
    def get_referenced_point_ids(self) -> builtins.set[types.EntityID]: ...
    def get_referenced_entity_ids(self) -> builtins.set[types.EntityID]: ...
    def is_mirror_compatible(self) -> builtins.bool: ...
    def mirror(self, axis: typing.Any) -> None: ...
    def get_draggable_point(self) -> typing.Optional[types.EntityID]: ...
    def seed_reference_attrs(self) -> None: ...
    def __deepcopy__(self, memo: dict) -> Constraint: ...
    def __copy__(self) -> Constraint: ...
"""

CONSTRAINT_STRIP_PROPS = (
    "value", "expression", "status", "user_visible",
    "p1", "p2", "p3", "p4", "p_origin", "p_width", "p_height",
    "entity_id", "circle_id", "line_id", "shape_id", "e1_id", "e2_id",
    "point_id", "center", "axis", "entity_ids",
)
CONSTRAINT_STRIP_METHODS = (
    "error", "gradient", "is_hit", "draw", "to_dict",
    "targets_segment", "constrains_radius", "get_referenced_point_ids",
    "get_referenced_entity_ids", "depends_on_points",
    "depends_on_entities", "update_from_context", "mirror",
    "is_world_anchored", "is_mirror_compatible", "get_draggable_point",
    "get_title", "get_subtitle", "get_edit_subtitle", "get_type_key",
    "get_type_name", "can_apply_to", "applies_to_entities",
    "seed_reference_attrs", "set_value", "set_expression",
    "set_user_visible", "__deepcopy__", "__copy__",
)
ENTITY_STRIP_PROPS = (
    "id", "type", "construction", "invisible", "array_copy",
    "constrained", "fill_color",
)
ENTITY_STRIP_METHODS = (
    "get_point_ids", "get_endpoint_ids", "get_junction_point_ids",
    "get_helper_ids", "clear_helper_references", "restore_base_flags",
    "supports_fill", "supports_point_on_curve", "is_radius_entity",
    "is_closed_loop", "is_edge_entity", "characteristic_length",
    "characteristic_length_pairs", "enclosed_signed_area",
    "update_constrained_status", "mirror", "transform_offsets",
    "rewrite_offsets_from", "as_offset_item", "geometry_signature",
    "to_dict", "get_state", "set_state", "to_geometry", "to_polyline",
    "to_polylines", "to_polygon_vertices", "append_to_geometry",
    "create_fill_geometry", "hit_test", "contains_point",
    "is_contained_by", "intersects_rect", "signed_distance_to",
    "tangent_at", "__deepcopy__", "__copy__", "get_drag_anchor_points",
    "get_rigidly_connected_points", "get_all_frame_point_ids",
    "get_ignorable_unconstrained_points", "plan_offset",
)


def rewrite_self_returns(path):
    text = path.read_text()
    pattern = re.compile(
        r"(    def (?:__new__|from_dict)\(cls, [^\n]*\)) -> [A-Z][A-Za-z0-9_]*:"
    )
    new_text, n = pattern.subn(r"\1 -> typing.Self:", text)
    if n:
        path.write_text(new_text)
        print("Self returns:", path.parent.name, n)


def member_end(lines, i):
    """Returns the index of the first line after the member starting
    at `i` that is not part of its definition."""
    j = i + 1
    while j < len(lines):
        stripped = lines[j].rstrip("\n")
        if stripped.strip() == "":
            return j
        if stripped.startswith("    @") or stripped.startswith("    def "):
            return j
        if not stripped.startswith("    "):
            return j
        j += 1
    return len(lines)


def strip_inherited(path, props, methods):
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    out = []
    i = 0
    removed = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        replaced = False
        if stripped.startswith("def ") or stripped.startswith("@property"):
            name = None
            if stripped.startswith("def "):
                for name in methods:
                    if stripped.startswith("def " + name + "("):
                        break
                else:
                    name = None
            else:
                if i + 1 < len(lines):
                    for name in props:
                        if ("def " + name + "(") in lines[i + 1]:
                            break
                    else:
                        name = None
            if name is not None:
                j = member_end(lines, i)
                removed += 1
                i = j
                replaced = True
        if not replaced:
            out.append(line)
            i += 1
    if removed:
        path.write_text("".join(out))
        print("strip:", path.parent.name, removed)


def merge(path, class_name, members):
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    out = []
    in_target = False
    done = False
    for line in lines:
        if not done and line.strip() == "class " + class_name + ":":
            in_target = True
            out.append(line)
            continue
        if in_target and line == "    ...\n":
            out.append(members)
            in_target = False
            done = True
            continue
        out.append(line)
    if not done:
        return False
    path.write_text("".join(out))
    print("overlay:", path.parent.name, class_name)
    return True


def main():
    for path in sorted((STUBS / "constraints").glob("*/__init__.pyi")):
        rewrite_self_returns(path)
        strip_inherited(path, CONSTRAINT_STRIP_PROPS,
                        CONSTRAINT_STRIP_METHODS)
    for path in sorted((STUBS / "entities").glob("*/__init__.pyi")):
        rewrite_self_returns(path)
        strip_inherited(path, ENTITY_STRIP_PROPS, ENTITY_STRIP_METHODS)
    merge(STUBS / "entities" / "__init__.pyi", "Entity", ENTITY_MEMBERS)
    merge(STUBS / "constraints" / "__init__.pyi", "Constraint",
          CONSTRAINT_MEMBERS)


if __name__ == "__main__":
    main()


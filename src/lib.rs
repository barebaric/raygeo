use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3_stub_gen::{StubInfo, StubGenConfig};

mod geo;
mod ops;

#[pymodule(gil_used = false)]
fn raygeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr(
        "__doc__",
        concat!(
            "RayGeo — 2D/3D geometry engine for laser cutting and CAM applications.\n",
            "\n",
            "Core features:\n",
            "- Geometry types: points, lines, arcs, circles, beziers, polygons, rectangles\n",
            "- Path analysis: length, area, bounding box, containment, intersection\n",
            "- Path manipulation: offset, clipping, fitting, simplification, smoothing\n",
            "- Minkowski sums for toolpath generation\n",
            "- Command sequence (Ops) for laser cutter motion control\n",
            "- Serialization to/from industry formats\n",
            "\n",
            "Submodules:\n",
            "- raygeo.geo — Geometry and path/shape/algo operations\n",
            "- raygeo.ops — Command sequence (Ops) manipulation\n",
            "\n",
            "Examples:\n",
            "    Creating and inspecting geometry:\n",
            "\n",
            "    >>> from raygeo import Geometry\n",
            "    >>> geom = Geometry()\n",
            "    >>> geom.add_rect(0, 0, 100, 50)\n",
            "    >>> geom.add_circle(50, 25, 10)\n",
            "    >>> geom.area()\n",
            "    5000.0 - 314.159...\n",
            "    >>> len(geom)\n",
            "    2\n",
            "\n",
            "    Manipulating command sequences:\n",
            "\n",
            "    >>> from raygeo.ops import Ops, Command\n",
            "    >>> ops = Ops()\n",
            "    >>> ops.set_speed(100)\n",
            "    >>> ops.move_to(0, 0)\n",
            "    >>> ops.line_to(100, 0)\n",
            "    >>> ops.travel_distance()\n",
            "    100.0",
        ),
    )?;
    geo::register(m)?;
    ops::register(m)?;
    // Backward-compat re-exports on root
    m.add("Geometry", m.getattr("geo")?.getattr("Geometry")?)?;
    m.add("Ops", m.getattr("ops")?.getattr("Ops")?)?;
    m.add_function(wrap_pyfunction!(generate_stubs, m)?)?;
    Ok(())
}

#[pyfunction]
fn generate_stubs(path: &str) -> PyResult<()> {
    let stub_info = StubInfo::from_project_root("raygeo".to_string(), path.into(), false, StubGenConfig::default())
        .map_err(|e| PyRuntimeError::new_err(format!("StubInfo failed: {}", e)))?;
    let module_docs: std::collections::HashMap<&str, &str> = [
        ("raygeo", "\
RayGeo — 2D/3D geometry engine for laser cutting and CAM applications.

Core features:
- Geometry types: points, lines, arcs, circles, beziers, polygons, rectangles
- Path analysis: length, area, bounding box, containment, intersection
- Path manipulation: offset, clipping, fitting, simplification, smoothing
- Minkowski sums for toolpath generation
- Command sequence (Ops) for laser cutter motion control
- Serialization to/from industry formats

Submodules:
- raygeo.geo — Geometry and path/shape/algo operations
- raygeo.ops — Command sequence (Ops) manipulation

Examples:
    Creating and inspecting geometry:

    >>> from raygeo import Geometry
    >>> geom = Geometry()
    >>> geom.add_rect(0, 0, 100, 50)
    >>> geom.add_circle(50, 25, 10)
    >>> geom.area()
    5000.0 - 314.159...
    >>> len(geom)
    2

    Manipulating command sequences:

    >>> from raygeo.ops import Ops, Command
    >>> ops = Ops()
    >>> ops.set_speed(100)
    >>> ops.move_to(0, 0)
    >>> ops.line_to(100, 0)
    >>> ops.travel_distance()
    100.0
"),
        ("raygeo.geo", "\
Geometry types and operations for 2D/3D path data.

Provides the Geometry class, path submodules for analysis/cleanup/intersection,
shape submodules (arc, bezier, circle, line, polygon, rect, point),
and algorithm submodules (clipping, fitting, minkowski, simplify, smooth).
"),
        ("raygeo.ops", "\
Command sequence (Ops) manipulation for laser cutter motion control.

Provides Ops — a container of command primitives (move, line, arc, bezier,
state changes) with methods for transformation, clipping, linearization,
timing estimation, serialization, and more.
"),
    ].into_iter().collect();
    let transform_matrix_doc = "\
r\"\"\"4x4 affine transformation matrix for 2D/3D coordinate transforms.

Layout (row-major):

```
[ Rxx  Rxy  Rxz  Tx ]   row 0: X basis vector + X translation
[ Ryx  Ryy  Ryz  Ty ]   row 1: Y basis vector + Y translation
[ Rzx  Rzy  Rzz  Tz ]   row 2: Z basis vector + Z translation
[  0    0    0   1  ]   row 3: homogeneous row (identity)
```

For 2D transforms, set the Z components to identity:
  ``Rzx = Rzy = 0.0``, ``Rzz = 1.0``, ``Tz = 0.0``
\"\"\"
";

    for module in stub_info.modules.values() {
        let mut content = module.format_with_config(false);
        if let Some(doc) = module_docs.get(module.name.as_str()) {
            let doc_block = format!(
                "r\"\"\"{}\n\"\"\"\n\n",
                doc.trim_end()
            );
            content = doc_block + &content;
        }
        // Inject documentation for TransformMatrix type alias
        if module.name == "raygeo.geo.types" {
            let target = "TransformMatrix: TypeAlias = ";
            if let Some(pos) = content.find(target) {
                let end_of_line = content[pos..].find('\n').unwrap_or(0);
                let insert_pos = pos + end_of_line + 1;
                content.insert_str(insert_pos, transform_matrix_doc);
            }
        }
        let parts: Vec<&str> = module.name.split('.').collect();
        if parts.len() == 1 {
            // Root module: raygeo -> __init__.pyi
            // Backward-compat re-exports
            content.push_str("from .geo import Geometry\n");
            content.push_str("from .ops import Ops\n");
            std::fs::write(format!("{}/__init__.pyi", path), content)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        } else {
            // Strip "raygeo." prefix to get relative path components
            let rel_parts = &parts[1..];
            let has_submodules = stub_info.modules.values().any(|m| {
                m.name != module.name && m.name.starts_with(&format!("{}.", module.name))
            });
            if has_submodules {
                // Intermediate module -> directory with __init__.pyi
                let subdir = format!("{}/{}", path, rel_parts.join("/"));
                std::fs::create_dir_all(&subdir)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
                std::fs::write(format!("{}/__init__.pyi", subdir), content)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            } else {
                // Leaf module -> .pyi file in parent directory
                let parent = &rel_parts[..rel_parts.len() - 1];
                let filename = rel_parts.last().unwrap();
                let subdir = if parent.is_empty() {
                    path.to_string()
                } else {
                    format!("{}/{}", path, parent.join("/"))
                };
                std::fs::create_dir_all(&subdir)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
                std::fs::write(format!("{}/{}.pyi", subdir, filename), content)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            }
        }
    }
    Ok(())
}

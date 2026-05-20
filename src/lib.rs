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
    for module in stub_info.modules.values() {
        let mut content = module.format_with_config(false);
        if let Some(doc) = module_docs.get(module.name.as_str()) {
            let doc_block = format!(
                "r\"\"\"{}\n\"\"\"\n\n",
                doc.trim_end()
            );
            content = doc_block + &content;
        }
        let parts: Vec<&str> = module.name.split('.').collect();
        if parts.len() == 1 {
            // Root module: raygeo -> __init__.pyi
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

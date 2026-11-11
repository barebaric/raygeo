//! Python bindings for sketch topology (chain assembly and
//! closed-loop detection).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.topology",
    name = "Edge"
)]
#[derive(Clone, PartialEq)]
pub struct PySketchEdge {
    /// Unique entity id.
    #[pyo3(get)]
    pub entity_id: u64,
    /// First endpoint point id.
    #[pyo3(get)]
    pub p1: u64,
    /// Second endpoint point id.
    #[pyo3(get)]
    pub p2: u64,
    /// atan2 of the entity tangent leaving from ``p1`` (entity-kind
    /// specific, precomputed by the caller).
    #[pyo3(get)]
    pub angle_at_p1: f64,
    /// atan2 of the entity tangent leaving from ``p2``.
    #[pyo3(get)]
    pub angle_at_p2: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySketchEdge {
    /// Creates an edge descriptor.
    #[new]
    #[pyo3(signature = (entity_id, p1, p2, angle_at_p1, angle_at_p2))]
    fn new(
        entity_id: u64,
        p1: u64,
        p2: u64,
        angle_at_p1: f64,
        angle_at_p2: f64,
    ) -> Self {
        PySketchEdge {
            entity_id,
            p1,
            p2,
            angle_at_p1,
            angle_at_p2,
        }
    }
}

#[gen_stub_pyclass_complex_enum]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.topology",
    name = "LoopShape"
)]
#[derive(Clone, PartialEq)]
pub enum PyLoopShape {
    /// An arc edge, contributing the circular-segment area correction.
    Arc {
        entity_id: u64,
        start: u64,
        end: u64,
        center: u64,
        clockwise: bool,
    },
    /// A closed single-entity loop (circle, ellipse) reported as its
    /// own face with the caller-computed enclosed signed area.
    Closed { entity_id: u64, signed_area: f64 },
}

pyo3_stub_gen::module_doc!("raygeo.sketch.topology", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Sketch topology: chain assembly and closed-loop detection.

plan_chains partitions two-endpoint edges into continuous chains of
(entity_id, forward) tuples by collapsing coincident points and
growing paths from each unvisited edge - the topology behind
sketch-to-geometry conversion. find_closed_loops traces the faces
of the sketch graph via half-edge traversal with tangent-sorted
outgoing edges, keeping positively oriented loops and appending
closed single-entity loops. Geometry emission stays with the
caller; the tangent angles the traversal needs are precomputed
inputs.
";

use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::sketch::topology::{
    find_closed_loops, plan_chains, ClosedEntity, SketchArc, SketchEdge,
};

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def plan_chains(
        edges: list[raygeo.sketch.topology.Edge],
        coincident_pairs: list[tuple[int, int]],
    ) -> list[list[tuple[int, bool]]]:
        """Partition edges into continuous chains.

        Collapses coincident points into shared nodes, then grows a
        path from each unvisited edge in both directions, always
        taking the first unvisited continuation.

        :param edges: Edge descriptors with endpoint point ids and
            precomputed leaving tangent angles.
        :param coincident_pairs: Coincident point id pairs.
        :returns: Chains of ``(entity_id, forward)`` tuples.
        :complexity: O(e^2) worst case, typically near-linear
        """
"#,
    module = "raygeo.sketch.topology"
)]
#[pyfunction(name = "plan_chains")]
fn plan_chains_py(
    edges: Vec<PySketchEdge>,
    coincident_pairs: Vec<(u64, u64)>,
) -> Vec<Vec<(u64, bool)>> {
    let edge_defs: Vec<SketchEdge> = edges
        .iter()
        .map(|e| SketchEdge {
            entity_id: e.entity_id,
            p1: e.p1,
            p2: e.p2,
            angle_at_p1: e.angle_at_p1,
            angle_at_p2: e.angle_at_p2,
        })
        .collect();
    plan_chains(&edge_defs, &coincident_pairs)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def find_closed_loops(
        edges: list[raygeo.sketch.topology.Edge],
        points: list[tuple[int, float, float]],
        coincident_groups: list[list[int]],
        loop_shapes: list[raygeo.sketch.topology.LoopShape],
    ) -> list[list[tuple[int, bool]]]:
        """Find all closed loops (faces) of the sketch graph.

        Traces faces via half-edge traversal with tangent-sorted
        outgoing edges, keeping loops with positive signed area
        (> 1e-6, refined by circular-segment corrections for arcs)
        and appending closed single-entity loops.

        :param edges: Edge descriptors.
        :param points: Point id and coordinates ``(id, x, y)``.
        :param coincident_groups: Transitive coincident groups
            (each group listing its member point ids).
        :param loop_shapes: Arc descriptors and closed single-entity
            loops contributing to the loop detection.
        :returns: Loops of ``(entity_id, forward)`` tuples.
        :complexity: O(e log e) per traversal step
        """
"#,
    module = "raygeo.sketch.topology"
)]
#[pyfunction(name = "find_closed_loops")]
fn find_closed_loops_py(
    edges: Vec<PySketchEdge>,
    points: Vec<(u64, f64, f64)>,
    coincident_groups: Vec<Vec<u64>>,
    loop_shapes: Vec<PyLoopShape>,
) -> Vec<Vec<(u64, bool)>> {
    let edge_defs: Vec<SketchEdge> = edges
        .iter()
        .map(|e| SketchEdge {
            entity_id: e.entity_id,
            p1: e.p1,
            p2: e.p2,
            angle_at_p1: e.angle_at_p1,
            angle_at_p2: e.angle_at_p2,
        })
        .collect();
    let mut arcs: Vec<SketchArc> = Vec::new();
    let mut closed: Vec<ClosedEntity> = Vec::new();
    for shape in &loop_shapes {
        match shape {
            PyLoopShape::Arc {
                entity_id,
                start,
                end,
                center,
                clockwise,
            } => arcs.push(SketchArc {
                entity_id: *entity_id,
                start: *start,
                end: *end,
                center: *center,
                clockwise: *clockwise,
            }),
            PyLoopShape::Closed {
                entity_id,
                signed_area,
            } => {
                closed.push(ClosedEntity {
                    entity_id: *entity_id,
                    signed_area: *signed_area,
                });
            }
        }
    }
    find_closed_loops(&edge_defs, &points, &coincident_groups, &arcs, &closed)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let topology_mod = PyModule::new(py, "topology")?;

    topology_mod.setattr("__doc__", MODULE_DOC)?;

    topology_mod.add_class::<PySketchEdge>()?;
    topology_mod.add_class::<PyLoopShape>()?;

    register_functions!(topology_mod, plan_chains_py, find_closed_loops_py,);

    m.add_submodule(&topology_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.topology", &topology_mod)?;

    Ok(())
}

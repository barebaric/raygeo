//! Connected components of the sketch constraint graph.
//!
//! The constraint system decouples exactly along these components:
//! parameters are constant during a solve, so no constraint couples
//! points across components.  Interactive drags can therefore restrict
//! the solve to the component containing the dragged geometry.

use std::collections::{BTreeSet, HashMap};

use super::constraint::SketchConstraint;
use crate::sketch::union_find::UnionFind;

/// Partitions all points referenced by constraints into connected
/// components of the constraint graph.  Two points share a component
/// when some chain of constraints couples them.  Points not referenced
/// by any constraint belong to no component.
pub fn compute_constraint_components(
    registry: &crate::sketch::registry::EntityRegistry,
    constraints: &[&dyn SketchConstraint],
) -> Vec<BTreeSet<u64>> {
    let mut parent = UnionFind::new();

    for constraint in constraints {
        let pids = constraint.referenced_points(registry);
        let first = match pids.first() {
            Some(&id) => id,
            None => continue,
        };
        parent.ensure(first);
        for pid in pids.iter().skip(1) {
            parent.ensure(*pid);
            parent.union(*pid, first);
        }
    }

    let mut components: HashMap<u64, BTreeSet<u64>> = HashMap::new();
    let mut roots: Vec<u64> = Vec::new();
    let ids: Vec<u64> = parent.ids().collect();
    for pid in ids {
        let root = parent.find(pid);
        components
            .entry(root)
            .or_insert_with(|| {
                roots.push(root);
                BTreeSet::new()
            })
            .insert(pid);
    }
    roots
        .into_iter()
        .map(|root| components.remove(&root).expect("root present"))
        .collect()
}

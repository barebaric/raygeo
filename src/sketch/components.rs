//! Connected components of the sketch constraint graph.
//!
//! The constraint system decouples exactly along these components:
//! parameters are constant during a solve, so no constraint couples
//! points across components.  Interactive drags can therefore restrict
//! the solve to the component containing the dragged geometry.

use std::collections::{BTreeSet, HashMap};

use super::constraint::SketchConstraint;

/// Partitions all points referenced by constraints into connected
/// components of the constraint graph.  Two points share a component
/// when some chain of constraints couples them.  Points not referenced
/// by any constraint belong to no component.
pub fn compute_constraint_components(
    constraints: &[&dyn SketchConstraint],
) -> Vec<BTreeSet<u64>> {
    let mut parent: HashMap<u64, u64> = HashMap::new();

    fn find(parent: &mut HashMap<u64, u64>, pid: u64) -> u64 {
        let mut root = pid;
        while let Some(&next) = parent.get(&root) {
            if next == root {
                break;
            }
            root = next;
        }
        let mut current = pid;
        while let Some(&next) = parent.get(&current) {
            if next == current {
                break;
            }
            parent.insert(current, root);
            current = next;
        }
        root
    }

    for constraint in constraints {
        let pids = constraint.referenced_points();
        let first = match pids.first() {
            Some(&id) => id,
            None => continue,
        };
        parent.entry(first).or_insert(first);
        for pid in pids.iter().skip(1) {
            parent.entry(*pid).or_insert(*pid);
            let root_first = find(&mut parent, first);
            let root_pid = find(&mut parent, *pid);
            if root_first != root_pid {
                parent.insert(root_pid, root_first);
            }
        }
    }

    let mut components: HashMap<u64, BTreeSet<u64>> = HashMap::new();
    let mut roots: Vec<u64> = Vec::new();
    let ids: Vec<u64> = parent.keys().copied().collect();
    for pid in ids {
        let root = find(&mut parent, pid);
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

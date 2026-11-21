//! Geometry emission for sketch documents.

use std::collections::{HashMap, HashSet};

use crate::geo::geometry::Geometry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::coincident::CoincidentConstraint;
use crate::sketch::entities::entity::Entity;
use crate::sketch::entities::text_box::TextBoxEntity;
use crate::sketch::registry::EntityRegistry;

/// Converts a sketch registry into geometry, linking chainable
/// entities into continuous paths.
pub fn document_geometry(
    registry: &EntityRegistry,
    constraints: &[Box<dyn SketchConstraint>],
    resolved_text: &HashMap<u64, Option<String>>,
) -> Geometry {
    let mut geo = Geometry::new();
    let mut chainable: Vec<&dyn Entity> = Vec::new();
    let mut standalone: Vec<&dyn Entity> = Vec::new();

    for entity in registry.all_entities() {
        let base = entity.base();
        if base.construction || base.invisible {
            continue;
        }
        if entity.is_edge_entity() {
            chainable.push(entity.as_ref());
        } else {
            standalone.push(entity.as_ref());
        }
    }

    for entity in standalone {
        if let Some(text_box) = entity.as_any().downcast_ref::<TextBoxEntity>()
        {
            let resolved = resolved_text
                .get(&text_box.base.id)
                .and_then(|value| value.as_deref());
            geo.extend(&text_box.to_geometry(registry, resolved));
        } else {
            geo.extend(&entity.to_geometry(registry));
        }
    }

    if chainable.is_empty() {
        return geo;
    }

    let mut parent: HashMap<u64, u64> = registry
        .point_ids()
        .into_iter()
        .map(|id| (id, id))
        .collect();
    for constraint in constraints {
        let Some(coincident) =
            constraint.as_any().downcast_ref::<CoincidentConstraint>()
        else {
            continue;
        };
        if parent.contains_key(&coincident.p1)
            && parent.contains_key(&coincident.p2)
        {
            union(&mut parent, coincident.p1, coincident.p2);
        }
    }

    let endpoints: Vec<(u64, u64)> = chainable
        .iter()
        .map(|entity| {
            let ids = entity.get_endpoint_ids();
            if ids.len() == 2 {
                (find(&mut parent, ids[0]), find(&mut parent, ids[1]))
            } else {
                (u64::MAX, u64::MAX)
            }
        })
        .collect();

    let mut adj: HashMap<u64, Vec<(usize, u64)>> = HashMap::new();
    for (index, (u, v)) in endpoints.iter().enumerate() {
        if *u == u64::MAX {
            continue;
        }
        adj.entry(*u).or_default().push((index, *v));
        adj.entry(*v).or_default().push((index, *u));
    }

    let mut visited: HashSet<usize> = HashSet::new();
    for start in 0..chainable.len() {
        if visited.contains(&start) {
            continue;
        }
        let (u, v) = endpoints[start];
        if u == u64::MAX {
            continue;
        }
        visited.insert(start);

        let mut right: Vec<(usize, bool)> = Vec::new();
        let mut current = v;
        while let Some((candidate, next)) =
            next_unvisited(&adj, &visited, current)
        {
            visited.insert(candidate);
            let candidate_u = endpoints[candidate].0;
            right.push((candidate, candidate_u == current));
            current = next;
        }

        let mut left: Vec<(usize, bool)> = Vec::new();
        current = u;
        while let Some((candidate, next)) =
            next_unvisited(&adj, &visited, current)
        {
            visited.insert(candidate);
            let candidate_u = endpoints[candidate].0;
            let candidate_v = endpoints[candidate].1;
            let is_forward = if candidate_u == next && candidate_v == current {
                true
            } else if candidate_u == current && candidate_v == next {
                false
            } else {
                candidate_u == next
            };
            left.push((candidate, is_forward));
            current = next;
        }

        let mut chain = Vec::new();
        chain.extend(left.into_iter().rev());
        chain.push((start, true));
        chain.extend(right);

        let (first, first_forward) = chain[0];
        let first_ids = chainable[first].get_endpoint_ids();
        let start_id = if first_forward {
            first_ids.first().copied()
        } else {
            first_ids.get(1).copied()
        };
        let Some((x, y)) = start_id.and_then(|id| registry.point_position(id))
        else {
            continue;
        };
        geo.move_to(x, y, 0.0);
        for (index, forward) in chain {
            chainable[index].append_to_geometry(&mut geo, registry, forward);
        }
    }

    geo
}

fn find(parent: &mut HashMap<u64, u64>, id: u64) -> u64 {
    let mut root = id;
    while parent.get(&root).copied().unwrap_or(root) != root {
        root = parent.get(&root).copied().unwrap_or(root);
    }
    let mut current = id;
    while current != root {
        let next = parent.get(&current).copied().unwrap_or(root);
        parent.insert(current, root);
        current = next;
    }
    root
}

fn union(parent: &mut HashMap<u64, u64>, a: u64, b: u64) {
    let root_a = find(parent, a);
    let root_b = find(parent, b);
    if root_a != root_b {
        parent.insert(root_a, root_b);
    }
}

fn next_unvisited(
    adj: &HashMap<u64, Vec<(usize, u64)>>,
    visited: &HashSet<usize>,
    node: u64,
) -> Option<(usize, u64)> {
    adj.get(&node)?
        .iter()
        .find(|(index, _)| !visited.contains(index))
        .copied()
}

//! Union-find over point ids, shared by the topology, contour,
//! geometry-emission and constraint-component code paths.
//!
//! A single implementation keeps the four call sites from drifting
//! apart: the same path-compression scheme, the same attachment rule
//! and deterministic iteration everywhere (the map is ordered, so
//! results no longer depend on hash-map iteration order).

use std::collections::BTreeMap;

/// Disjoint-set forest over `u64` ids.
///
/// Ids that were never inserted are their own root and stay
/// untracked.  `union(a, b)` attaches `a`'s root under `b`'s root —
/// the reference implementation's rule — so `find(b)`'s root wins.
pub(crate) struct UnionFind {
    parent: BTreeMap<u64, u64>,
}

impl UnionFind {
    pub(crate) fn new() -> Self {
        UnionFind {
            parent: BTreeMap::new(),
        }
    }

    /// Builds a forest with every given id as its own root.
    pub(crate) fn with_ids<I>(ids: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        UnionFind {
            parent: ids.into_iter().map(|id| (id, id)).collect(),
        }
    }

    /// Tracks `id` as its own root when not seen before.
    pub(crate) fn ensure(&mut self, id: u64) {
        self.parent.entry(id).or_insert(id);
    }

    /// Whether the id is tracked.
    pub(crate) fn contains(&self, id: u64) -> bool {
        self.parent.contains_key(&id)
    }

    /// The tracked ids, in ascending order.
    pub(crate) fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.parent.keys().copied()
    }

    /// The root of `id`'s set, with path compression.  Unknown ids
    /// are their own root.
    pub(crate) fn find(&mut self, id: u64) -> u64 {
        let mut root = id;
        while let Some(&next) = self.parent.get(&root) {
            if next == root {
                break;
            }
            root = next;
        }
        let mut current = id;
        while let Some(&next) = self.parent.get(&current) {
            if next == current {
                break;
            }
            self.parent.insert(current, root);
            current = next;
        }
        root
    }

    /// Merges the sets of `a` and `b`: `a`'s root attaches under
    /// `b`'s root, so `b`'s root represents the merged set.
    pub(crate) fn union(&mut self, a: u64, b: u64) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a != root_b {
            self.parent.insert(root_a, root_b);
        }
    }

    /// Like [`UnionFind::union`], but a no-op when either id is not
    /// tracked.
    pub(crate) fn union_if_present(&mut self, a: u64, b: u64) {
        if !self.contains(a) || !self.contains(b) {
            return;
        }
        self.union(a, b);
    }
}

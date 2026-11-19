//! Selection state of the sketch editor.
//!
//! The selection itself is host-independent data: point ids, entity ids,
//! a constraint index and an optional junction point. UI events and
//! signals stay with the host.

use crate::sketch::registry::EntityRegistry;

/// The current editor selection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub point_ids: Vec<u64>,
    pub entity_ids: Vec<u64>,
    pub constraint_idx: Option<usize>,
    pub junction_pid: Option<u64>,
}

impl Selection {
    /// Clears every selected item.
    pub fn clear(&mut self) {
        self.point_ids.clear();
        self.entity_ids.clear();
        self.constraint_idx = None;
        self.junction_pid = None;
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.point_ids.is_empty()
            && self.entity_ids.is_empty()
            && self.constraint_idx.is_none()
            && self.junction_pid.is_none()
    }

    fn update_list(collection: &mut Vec<u64>, item_id: u64, is_multi: bool) {
        if is_multi {
            if let Some(index) = collection.iter().position(|id| *id == item_id)
            {
                collection.remove(index);
            } else {
                collection.push(item_id);
            }
        } else if !collection.contains(&item_id) {
            collection.clear();
            collection.push(item_id);
        }
    }

    /// Selects a constraint by index.
    pub fn select_constraint(&mut self, idx: usize, is_multi: bool) {
        self.constraint_idx = Some(idx);
        if !is_multi {
            self.point_ids.clear();
            self.entity_ids.clear();
            self.junction_pid = None;
        }
    }

    /// Selects an implicit junction point.
    pub fn select_junction(&mut self, pid: u64, is_multi: bool) {
        self.junction_pid = Some(pid);
        if !is_multi {
            self.point_ids.clear();
            self.entity_ids.clear();
            self.constraint_idx = None;
        }
    }

    /// Selects a point by id.
    pub fn select_point(&mut self, pid: u64, is_multi: bool) {
        Self::update_list(&mut self.point_ids, pid, is_multi);
        if !is_multi {
            self.entity_ids.clear();
            self.constraint_idx = None;
            self.junction_pid = None;
        }
    }

    /// Selects an entity by id.
    pub fn select_entity(&mut self, entity_id: u64, is_multi: bool) {
        Self::update_list(&mut self.entity_ids, entity_id, is_multi);
        if !is_multi {
            self.point_ids.clear();
            self.constraint_idx = None;
            self.junction_pid = None;
        }
    }

    /// Adds every entity connected to the given entity through shared
    /// points.
    pub fn select_connected_entities(
        &mut self,
        entity_id: u64,
        registry: &EntityRegistry,
    ) {
        let connected_ids = registry.connected_entity_ids(entity_id);
        for eid in connected_ids {
            if !self.entity_ids.contains(&eid) {
                self.entity_ids.push(eid);
            }
        }
        self.point_ids.clear();
        self.constraint_idx = None;
        self.junction_pid = None;
    }

    /// Adds one point to the selection without clearing other kinds.
    pub fn add_point(&mut self, pid: u64) {
        self.point_ids.push(pid);
    }

    /// Adds one entity to the selection without clearing other kinds.
    pub fn add_entity(&mut self, entity_id: u64) {
        self.entity_ids.push(entity_id);
    }

    /// Clears only the selected points.
    pub fn clear_points(&mut self) {
        self.point_ids.clear();
    }

    /// Clears only the selected entities.
    pub fn clear_entities(&mut self) {
        self.entity_ids.clear();
    }
}

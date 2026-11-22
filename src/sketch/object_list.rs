//! A simple vector-backed collection used by the sketch document.
//!
//! The document owns its constraints and arrays in these lists.  The
//! type is deliberately host-agnostic: it has no PyO3 or UI
//! dependencies, and the binding layer adds the Python sequence
//! protocol on top of it.

use std::sync::{Arc, RwLock};

/// A collection shared between the document and host views.
pub type SharedList<T> = Arc<RwLock<ObjectList<T>>>;

/// Creates an empty shared collection.
pub fn shared_list<T>() -> SharedList<T> {
    Arc::new(RwLock::new(ObjectList::new()))
}

/// A vector-backed collection with stable, explicit operations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObjectList<T> {
    items: Vec<T>,
}

impl<T> ObjectList<T> {
    /// Creates an empty collection.
    pub fn new() -> Self {
        ObjectList { items: Vec::new() }
    }

    /// Creates a collection from existing items.
    pub fn from_vec(items: Vec<T>) -> Self {
        ObjectList { items }
    }

    /// Consumes the collection and returns its items.
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Borrows the items.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Mutably borrows the items.
    pub fn items_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
    }

    /// Borrows one item.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// Mutably borrows one item.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    /// Appends an item.
    pub fn push(&mut self, item: T) {
        self.items.push(item);
    }

    /// Appends all items from another collection.
    pub fn extend(&mut self, items: impl IntoIterator<Item = T>) {
        self.items.extend(items);
    }

    /// Inserts an item at the given index, clamped to the length.
    pub fn insert(&mut self, index: usize, item: T) {
        let index = index.min(self.items.len());
        self.items.insert(index, item);
    }

    /// Removes and returns the item at the given index.
    pub fn remove(&mut self, index: usize) -> T {
        self.items.remove(index)
    }

    /// Removes the item at the given index, if it exists.
    pub fn remove_at(&mut self, index: usize) -> Option<T> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    /// Replaces the whole collection.
    pub fn replace(&mut self, items: Vec<T>) {
        self.items = items;
    }

    /// Removes every item.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Removes items selected by a predicate.
    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        self.items.retain(|item| keep(item));
    }
}

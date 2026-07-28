use std::any::Any;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub tag: String,
}

impl CacheKey {
    pub fn new(tag: impl Into<String>) -> Self {
        CacheKey { tag: tag.into() }
    }
}

#[derive(Debug)]
struct CacheEntry {
    value: Box<dyn Any + Send + Sync>,
    size_bytes: usize,
    generation: u64,
}

pub struct Cache {
    entries: HashMap<CacheKey, CacheEntry>,
    budget_bytes: usize,
    used_bytes: usize,
    clock: u64,
    insert_counter: u64,
    node_epochs: HashMap<String, u64>,
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("entries", &self.entries.len())
            .field("budget_bytes", &self.budget_bytes)
            .field("used_bytes", &self.used_bytes)
            .finish()
    }
}

impl Default for Cache {
    fn default() -> Self {
        Cache::new(2 * 1024 * 1024 * 1024)
    }
}

impl Cache {
    pub fn new(budget_bytes: usize) -> Self {
        Cache {
            entries: HashMap::new(),
            budget_bytes,
            used_bytes: 0,
            clock: 0,
            insert_counter: 0,
            node_epochs: HashMap::new(),
        }
    }

    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    pub fn budget_bytes(&self) -> usize {
        self.budget_bytes
    }

    /// Update the byte budget and evict entries until usage fits.
    pub fn set_budget_bytes(&mut self, new_budget: usize) {
        self.budget_bytes = new_budget;
        while self.used_bytes > self.budget_bytes {
            if !self.evict_one() {
                break;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(
        &mut self,
        key: &CacheKey,
    ) -> Option<&Box<dyn Any + Send + Sync>> {
        let clock = &mut self.clock;
        let entry = self.entries.get_mut(key)?;
        *clock += 1;
        entry.generation = *clock;
        Some(&entry.value)
    }

    /// Insert a cache entry.
    ///
    /// Returns ``true`` if the entry was inserted, ``false`` if the
    /// entry could not be stored because it exceeds the byte budget
    /// and eviction could not free enough space.
    pub fn insert(
        &mut self,
        key: CacheKey,
        value: Box<dyn Any + Send + Sync>,
        size_bytes: usize,
    ) -> bool {
        let tag = key.tag.clone();
        if let Some(old) = self.entries.remove(&key) {
            self.used_bytes -= old.size_bytes;
        }

        while self.used_bytes + size_bytes > self.budget_bytes {
            if !self.evict_one() {
                eprintln!(
                    "[raygeo] CACHE insert FAILED (eviction could not free enough) key={} size={} used={} budget={} entries={}",
                    tag, size_bytes, self.used_bytes, self.budget_bytes, self.entries.len(),
                );
                return false;
            }
        }

        self.clock += 1;
        self.insert_counter += 1;
        self.entries.insert(
            key,
            CacheEntry {
                value,
                size_bytes,
                generation: self.clock,
            },
        );
        self.used_bytes += size_bytes;
        true
    }

    fn evict_one(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let evict_key = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.generation)
            .map(|(k, _)| k.clone());
        if let Some(key) = evict_key {
            if let Some(entry) = self.entries.remove(&key) {
                self.used_bytes -= entry.size_bytes;
                return true;
            }
        }
        false
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.used_bytes = 0;
    }

    pub fn clear_prefix(&mut self, prefix: &str) {
        let keys_to_remove: Vec<CacheKey> = self
            .entries
            .keys()
            .filter(|k| k.tag.starts_with(prefix))
            .cloned()
            .collect();
        for key in keys_to_remove {
            if let Some(entry) = self.entries.remove(&key) {
                self.used_bytes -= entry.size_bytes;
            }
        }
    }

    pub fn bump_epoch(&mut self, key: &str) {
        let epoch = self.node_epochs.entry(key.to_string()).or_insert(0);
        *epoch += 1;
    }

    pub fn get_epoch(&self, key: &str) -> u64 {
        self.node_epochs.get(key).copied().unwrap_or(0)
    }

    pub fn remove_entry(&mut self, key: &str) {
        let cache_key = CacheKey {
            tag: key.to_string(),
        };
        if let Some(entry) = self.entries.remove(&cache_key) {
            self.used_bytes -= entry.size_bytes;
        }
    }
}

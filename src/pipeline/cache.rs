use std::any::Any;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub tag: String,
    pub payload_hash: u64,
}

impl CacheKey {
    pub fn new(tag: impl Into<String>, payload_hash: u64) -> Self {
        CacheKey {
            tag: tag.into(),
            payload_hash,
        }
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
        Cache::new(256 * 1024 * 1024)
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
        }
    }

    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    pub fn budget_bytes(&self) -> usize {
        self.budget_bytes
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

    pub fn insert(
        &mut self,
        key: CacheKey,
        value: Box<dyn Any + Send + Sync>,
        size_bytes: usize,
    ) {
        if let Some(old) = self.entries.remove(&key) {
            self.used_bytes -= old.size_bytes;
        }

        while self.used_bytes + size_bytes > self.budget_bytes {
            if !self.evict_one() {
                return;
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
}

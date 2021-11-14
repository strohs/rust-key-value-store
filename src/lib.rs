#![deny(missing_docs)]

use std::collections::HashMap;

pub struct KvStore {
    // in-memory KV store
    store: HashMap<String, String>,
}

impl KvStore {
    pub fn new() -> Self {
        KvStore {
            store: HashMap::new(),
        }
    }

    pub fn get(&self, key: String) -> Option<String> {
        self.store.get(&key).cloned()
    }

    pub fn set(&mut self, key: String, value: String) {
        self.store.insert(key, value);
    }

    pub fn remove(&mut self, key: String) {
        self.store.remove(&key);
    }
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}

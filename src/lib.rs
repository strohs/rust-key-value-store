#![deny(missing_docs)]
//! # KvStore
//! A multi-threaded, file-based, key-value store that maps [`String`] keys to [`String`] values
//!
//! [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
//!
use std::collections::HashMap;





/// the main structure used for working with a KvStore
pub struct KvStore {
    store: HashMap<String, String>,
}

impl KvStore {

    /// creates an empty KvStore with an initial capacity of 0
    /// # Example
    /// ```rust
    /// use kvs::KvStore;
    /// let mut kvs = KvStore::new();
    /// ```
    pub fn new() -> Self {
        KvStore {
            store: HashMap::new(),
        }
    }

    /// attempts to retrieve the value associated with `key`.
    /// returns `Some(value)` if the `key` was found, else returns `None`
    /// # Examples
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut kvs = KvStore::new();
    /// // inset the key "foo" with a value of "bar"
    /// kvs.set("foo".to_string(), "bar".to_string());
    /// // get the key "foo"
    /// assert_eq!(kvs.get("foo".to_string()), Some("bar".to_string()));
    /// ```
    pub fn get(&self, key: String) -> Option<String> {
        self.store.get(&key).cloned()
    }

    /// inserts the specified `key` and `value` into this `KvStore`, overriding any existing
    /// key/value entry
    /// # Examples
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut kvs = KvStore::new();
    /// // inset the key "foo" with a value of "bar"
    /// kvs.set("foo".to_string(), "bar".to_string());
    /// ```
    pub fn set(&mut self, key: String, value: String) {
        self.store.insert(key, value);
    }

    /// removes the specified `key` and its associated value from this KvStore
    /// # Examples
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut kvs = KvStore::new();
    /// // inset the key "foo" with a value of "bar"
    /// kvs.set("foo".to_string(), "bar".to_string());
    /// // get the key "foo"
    /// assert_eq!(kvs.get("foo".to_string()), Some("bar".to_string()));
    /// // remove "foo"
    /// kvs.remove("foo".to_string());
    /// assert_eq!(kvs.get("foo".to_string()), None);
    /// ```
    pub fn remove(&mut self, key: String) {
        self.store.remove(&key);
    }
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}

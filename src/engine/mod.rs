//! This module provides various key/value storage engine implementations.
use crate::Result;

/// A trait for the basic functionality of a key/value storage engine
pub trait KvsEngine {
    /// sets a `key` and `value`
    ///
    /// If the given `key` already exists the previous `value` will be overwritten.
    fn set(&mut self, key: String, value: String) -> Result<()>;

    /// Gets the value associated with the given `key`
    ///
    /// Returns `None` if the given `key` does not exist.
    fn get(&mut self, key: String) -> Result<Option<String>>;

    /// Removes the given `key` from the store
    ///
    /// # Errors
    ///
    /// Returns `KvsError::KeyNotFound` if the given `key` is not found.
    fn remove(&mut self, key: String) -> Result<()>;
}

mod kvs;

pub use self::kvs::KvStore;
//! This module provides various key/value storage engine implementations.
//! Currently, only the [`KvStore`] engine is implemented. In the future, a wrapper around the
//! [`sled`] database engine will be added.
//!
//! [`sled`]: https://docs.rs/sled/latest/sled/
use crate::Result;

/// A trait for the basic functionality of a key/value storage engine
pub trait KvsEngine: Clone + Send + 'static {
    /// sets a `key` and `value`
    ///
    /// If the given `key` already exists the previous `value` will be overwritten.
    fn set(&self, key: String, value: String) -> Result<()>;

    /// Gets the value associated with the given `key`
    ///
    /// Returns `None` if the given `key` does not exist.
    fn get(&self, key: String) -> Result<Option<String>>;

    /// Removes the given `key` (and associated value) from the store
    ///
    /// # Errors
    ///
    /// Returns `KvsError::KeyNotFound` if the given `key` is not found.
    fn remove(&self, key: String) -> Result<()>;
}



mod kvs;
//mod sled;

pub use self::kvs::KvStore;
//pub use self::sled::SledKvsEngine;
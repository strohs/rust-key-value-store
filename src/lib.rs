#![deny(missing_docs)]
//! A multi-threaded, file-based, key-value store that maps [`String`] keys to [`String`] values
//!
//! [`String`]: https://doc.rust-lang.org/std/string/struct.String.html

pub use error::Result;
pub use kvstore::KvStore;

mod error;
mod kvstore;

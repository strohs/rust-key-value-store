use thiserror::Error;
use std::io;
use serde_json;

/// type alias for all operations on a [`KvStore`] that could fail with an [`Error']
pub type Result<T> = std::result::Result<T, KvsError>;

/// Error variants used by ['KvsStore'].
/// It wraps any lower level errors from third party crates
#[derive(Error, Debug)]
pub enum KvsError {
    /// variant for errors caused by IO
    #[error("IO error")]
    Io {
        #[from]
        source: io::Error,
    },

    /// variant for errors when a key was not found in the KV Store
    #[error("Key not found")]
    KeyNotFound,

    /// variant for errors caused during type serialization/deserialization
    #[error("serialization/deserialization error")]
    Serialization(#[from] serde_json::Error),

    /// variant for errors when parsing strings to an integer type
    #[error("{}", .0)]
    Parsing(String),

    #[error("{}", .0)]
    Command(String),
}



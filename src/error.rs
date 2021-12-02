use serde_json;
use std::io;
use thiserror::Error;

/// type alias for all operations on a [`KvStore`] that could fail with an [`Error']
pub type Result<T> = std::result::Result<T, KvsError>;

/// Error variants used by ['KvsStore'].
/// It wraps any lower level errors from third party crates
#[derive(Error)]
pub enum KvsError {
    /// variant for errors caused by std::io
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

    /// variant for errors caused by an unknown or invalid command in the command log
    #[error("{}", .0)]
    Command(String),
}

impl std::fmt::Debug for KvsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by: {}", cause)?;
        current = cause.source();
    }
    Ok(())
}
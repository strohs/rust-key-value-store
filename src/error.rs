use std::io;
use thiserror::Error;
use std::string::FromUtf8Error;

/// type alias for all operations on a [`KvStore`] that could fail with an [`Error']
pub type Result<T> = std::result::Result<T, KvsError>;

/// Error variants used by ['KvsStore'].
/// It wraps any lower level errors from third party crates
#[derive(Error)]
pub enum KvsError {
    /// variant for errors caused by std::io
    #[error("IO error")]
    Io {
        /// source of the IO Error
        #[from]
        source: io::Error,
    },

    /// variant for errors when a key was not found in the KV Store
    #[error("Key not found")]
    KeyNotFound,

    /// variant for errors caused during type serialization/deserialization
    #[error("serialization/deserialization error")]
    Serialization(#[from] serde_json::Error),

    /// variant for errors when parsing strings to some other type
    #[error("{}", .0)]
    Parsing(String),

    /// variant for errors caused by an unknown or invalid command in the command log
    #[error("{}", .0)]
    InvalidCommand(String),

    /// catch-all variant for reporting error message strings to clients
    #[error("{}", .0)]
    StringErr(String),

    /// variant for sled related errors
    #[error("sled error")]
    Sled(#[from] sled::Error),

    /// a Key or value is an invalid UTF-8 sequence
    #[error("{}", .0)]
    Utf8Error(#[from] FromUtf8Error),

    /// variant for errors caused during type serialization/deserialization
    #[error("{}", .0)]
    Locking(String),
}

/// a custom Debug implementation that will write the entire error chain
impl std::fmt::Debug for KvsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

/// writes the entire error chain of the given error `e`, to the formatter.
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
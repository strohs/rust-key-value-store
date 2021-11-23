// use std::error::Error;
// use std::fmt;
// use std::fmt::{Display, Formatter};

/// type alias for all operations on a [`KvStore`] that could fail with an [`Error']
pub type Result<T> = std::result::Result<T, anyhow::Error>;

// /// The Error variants used by ['KvsStore'].
// /// It wraps any lower level errors from third party crates, and then uses the `anyhow` crate
// /// to attach context information
// #[derive(Debug)]
// pub enum KvsError {
//     /// variant for errors caused from file IO
//     IOError,
//
//     /// variant for errors when a key was not found in the KV Store
//     KeyNotFound,
//
//     /// Serde Error
//     SerializationError,
// }
//
// impl Display for KvsError {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         match self {
//             KvsError::IOError => write!(f, "IO Error"),
//             KvsError::KeyNotFound => write!(f, "Key not found"),
//             KvsError::SerializationError => write!(f, "Serialization Error"),
//         }
//     }
// }
//
// impl Error for KvsError {}

// impl From<std::io::Error> for KvsError {
//     fn from(e: std::io::Error) -> Self {
//         // todo map different io error variants?
//         KvsError::IOError(e.to_string())
//     }
// }
//
// impl From<serde_json::Error> for KvsError {
//     fn from(e: serde_json::Error) -> Self {
//         dbg!(&e);
//         KvsError::SerializationError
//     }
// }
//
// impl From<Utf8Error> for KvsError {
//     fn from(e: Utf8Error) -> Self {
//         KvsError::IOError(e.to_string())
//     }
// }


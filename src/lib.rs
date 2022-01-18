#![deny(missing_docs)]
//! A multi-threaded, persistent, key-value store (kvs), that maps [`String`] keys to [`String`] values.
//!
//! The kvs is implemented as a client/server application, using synchronous networking over a custom protocol to send/receive data to/from the KvStore.
//!
//! Fully functioning command-line executables are provided that can be used to start a [`kvs-server`] and then send commands to it using the [`kvs-client`].
//!
//! ## Supported Operations
//! The kvs supports three types of operations (a.k.a "commands"):
//!
//! - `GET` a value associated with a key from the store
//! - `SET` a key/value pair in the store
//! - `REMOVE` a key/value pair from the store
//! See the [`KvsEngine`] trait and the [`Request`] and [`Response`] types for more information on the structure of these operations.
//!
//! ## KvStore
//! [`KvStore`] is the primary structure that implements the functionality of the kvs engine.
//! It is responsible for the following tasks:
//! - processing the GET, SET and REMOVE operations
//! - maintaining kv data within an in-memory, concurrent HashMap
//! - persisting the kv data into "command-log" files
//! - loading kv data from the command-log files at start-up
//! - periodically performing a command-log clean-up (a.k.a a compaction) once the size of stale data hits a certian byte size
//!     - This compaction will run once the size of stale data hits the COMPACTION_THRESHOLD limit (currently set to 2 KB).
//!
//!
//! ## Client / Server
//! The [`client`] / [`server`] structs implement the client/server functionality.  They are responsible for the networking portion of this application, but also
//! handle the deserialization/serialization of data into a custom protocol.
//!
//! The custom protocol is simply a "GET", "SET" or "REMOVE" [`Request`] encoded to/from a JSON string, and then sent over the wire using a Rust TcpStream.
//! If the server was able to successfully service a [`Request`], then an "Ok" [`Response`] will be returned, containing the result of the request.
//! If an error occured, an [`Err`] response is returned, containing a description of the error.
//!
//!
//! ### Client / Server executables
//! Two command line executables are provided that can be used to interact with the ['KvStore'].
//! See [`kvs-server`] and [`kvs-client`] for info on their use.
//!
//!
//! ## Command Log Files
//! KV data is persisted into a series of "command log" files, that are created every time the KvStore is (re)started.  BY default, these files
//! are created in the same directory that you started the [`kvs-server`] from.
//! The files will have an integer file name (beginning with "1") and will end with a suffix of ".log". For example: 1.log, 2.log, etc...
//! The directory where these files are kept is specified when you create a new [`KvStore`].
//!
//! The .log files keep track of "SET" and "REMOVE" operations received by the KvStore. The operations themselves are just serialized
//! JSON strings.
//! "GET" commands are not persisted as they have no bearing on the current state of the store.
//!
//! [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
//! [`serde`]: https://serde.rs
//! [`client`]: ./struct.KvsClient.html
//! [`server`]: ./struct.KvsServer.html
//! [`KvsEngine`]: ./engine/trait.KvsEngine.html
//! [`Request`]: ./enum.Request.html
//! [`Response`]: ./enum.Response.html
//! [`kvs-server`]: ./kvs-server.rs
//! [`kvs-client`]: /kvs-client.rs


pub use error::{Result, KvsError};
pub use engine::{KvsEngine, KvStore};
pub use server::KvsServer;
pub use client::KvsClient;
pub use thread_pool::{ThreadPool, NaiveThreadPool, SharedQueueThreadPool, RayonThreadPool};
pub use command::{Response, Request};

mod client;
mod command;
mod engine;
mod error;
mod server;
pub mod thread_pool;
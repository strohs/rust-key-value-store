#![deny(missing_docs)]
//! A multi-threaded, file-based, key-value store that maps [`String`] keys to [`String`] values
//!
//! [`String`]: https://doc.rust-lang.org/std/string/struct.String.html

pub use error::{Result, KvsError};
pub use engine::{KvsEngine, KvStore};
pub use server::KvsServer;
pub use client::KvsClient;
pub use thread_pool::{ThreadPool, SharedQueueThreadPool};
pub use command::{Response, Request};

mod error;
mod command;
mod engine;
mod thread_pool;
mod server;
mod client;

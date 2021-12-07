#![deny(missing_docs)]
//! A multi-threaded, file-based, key-value store that maps [`String`] keys to [`String`] values
//!
//! [`String`]: https://doc.rust-lang.org/std/string/struct.String.html

pub use error::{Result, KvsError};
pub use engine::{KvsEngine, KvStore, SledKvsEngine};
pub use server::KvsServer;
pub use client::KvsClient;
pub use command::{Response, Request};

mod error;
mod command;
mod engine;
mod server;
mod client;

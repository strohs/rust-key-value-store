use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};
use serde::Deserialize;
use serde_json::de::IoRead;
use serde_json::Deserializer;
use crate::command::{Request, Response};
use crate::{KvsError, Result};

/// `KvsClient` contains the functionality for communication with a [`KvsServer`]
pub struct KvsClient {
    reader: Deserializer<IoRead<BufReader<TcpStream>>>,
    writer: BufWriter<TcpStream>,
}

impl KvsClient {

    /// creates a client and establishes a socket connection to the server at the given `addr`
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let tcp_reader = TcpStream::connect(addr)?;
        let tcp_writer = tcp_reader.try_clone()?;

        Ok(KvsClient {
            reader: Deserializer::from_reader(BufReader::new(tcp_reader)),
            writer: BufWriter::new(tcp_writer),
        })
    }

    /// gets the value of the specified `key` from the server
    /// ## Returns
    /// `Ok<Some<String>>` if the value was found for the key.
    /// `Ok<None>` if there is no value associated with the key
    /// `Err<KvsError::Command>` if an error occurred when retrieving the key
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        let req = Request::Get { key };
        serde_json::to_writer(&mut self.writer, &req)?;
        self.writer.flush()?;

        match Response::deserialize(&mut self.reader)? {
            Response::Ok(value) => Ok(value),
            Response::Err(msg) => Err(KvsError::StringErr(msg)), // re-throwing error here
        }
    }

    /// sends a set key/value request to the server
    /// # Returns
    /// `Ok<None>` if the the key/value pair was successfully set
    /// # Errors
    /// `Err<KvsError::StringErr>` if an error occurred while setting the key/value
    pub fn set(&mut self, key: String, value: String) -> Result<Option<String>> {
        let req = Request::Set { key, value };
        serde_json::to_writer(&mut self.writer, &req)?;
        self.writer.flush()?;

        match Response::deserialize(&mut self.reader)? {
            Response::Ok(_value) => Ok(None),
            Response::Err(msg) => Err(KvsError::StringErr(msg)),
        }
    }

    /// removes a key and its associated value from the store
    /// # Returns
    /// `Ok<None>` if the the key/value was removed
    /// # Errors
    /// `Err<KvsError::StringErr>` if an error occurred while attempting to remove the key
    pub fn remove(&mut self, key: String) -> Result<Option<String>> {
        let req = Request::Remove { key };
        serde_json::to_writer(&mut self.writer, &req)?;
        self.writer.flush()?;

        match Response::deserialize(&mut self.reader)? {
            Response::Ok(_value) => Ok(None),
            Response::Err(msg) => Err(KvsError::StringErr(msg)),
        }
    }
}
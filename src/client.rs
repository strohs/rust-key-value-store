use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};
use serde::Deserialize;
use serde_json::de::IoRead;
use serde_json::Deserializer;
use crate::command::{Request, Response};
use crate::{KvsError, Result};

/// The `KvsClient` struct is used to issue synchronous command [`Request`]s to a running [`KvsServer`].
///
/// It can issue "GET", "SET", and "REMOVE" operations, and then wait for (and parse) the [`Response`] from the server.
///
/// # Example
/// Connect to a KvsServer running at 127.0.0.1:4000 and then issue a "get" request to get the value
/// associated with the key "mykey".
/// ```rust
/// use kvs::KvsClient;
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #
///
/// // specify the IP address and port of a kvs-server
/// let server_addr = "127.0.0.1:4000";
/// let mut client = KvsClient::connect(server_addr)?;
///
/// // now try to get the value associated with a key named "mykey"
/// match client.get("mykey".to_string()) {
///     Ok(Some(value)) => println!("got value {}", value),
///     Ok(None) => println!("no value for key 'mykey'"),
///     Err(e) => println!("an error occurred {:?}", e),
/// }
///
/// #
/// # Ok(())
/// # }
/// ```
///
/// [`KvsServer`]: ../struct.KvsServer.html
/// [`Request`]: ./enum.Request
/// [`Response`]: ./enum.Response
pub struct KvsClient {
    reader: Deserializer<IoRead<BufReader<TcpStream>>>,
    writer: BufWriter<TcpStream>,
}

impl KvsClient {

    /// tries to create a KvsClient and establish a socket connection to a KvsServer running at
    /// the given `addr`
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let tcp_reader = TcpStream::connect(addr)?;
        let tcp_writer = tcp_reader.try_clone()?;

        Ok(KvsClient {
            reader: Deserializer::from_reader(BufReader::new(tcp_reader)),
            writer: BufWriter::new(tcp_writer),
        })
    }

    /// gets the value of the specified `key` from the server
    /// # Returns
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
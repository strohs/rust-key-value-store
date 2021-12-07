use crate::{KvsEngine, Result};
use crate::command::{Request, Response};
use serde_json::Deserializer;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use tracing::{debug, error};

/// The server of a key value store.
pub struct KvsServer<E: KvsEngine> {
    /// the storage engine being used
    engine: E,
}

impl<E: KvsEngine> KvsServer<E> {
    /// Create a `KvsServer` with a given storage engine.
    pub fn new(engine: E) -> Self {
        KvsServer { engine }
    }

    /// Run the server listening on the given address
    pub fn run<A: ToSocketAddrs>(mut self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.serve(stream) {
                        error!("Error on serving client: {}", e);
                    }
                }
                Err(e) => error!("Connection failed: {}", e),
            }
        }
        Ok(())
    }

    /// deserializes a request, executes a request in the KvsEngine, and returns a response to the client
    fn serve(&mut self, tcp: TcpStream) -> Result<()> {
        let peer_addr = tcp.peer_addr()?;
        let reader = BufReader::new(&tcp);
        let mut writer = BufWriter::new(&tcp);
        let req_reader = Deserializer::from_reader(reader).into_iter::<Request>();

        let mut send_resp = move |resp: Response| -> Result<()> {
            serde_json::to_writer(&mut writer, &resp)?;
            writer.flush()?;
            debug!("Response sent to {}: {:?}", peer_addr, resp);
            Ok(())
        };

        for req in req_reader {
            let req = req?;
            debug!("Receive request from {}: {:?}", peer_addr, req);
            match req {
                Request::Get { key } => match self.engine.get(key) {
                    Ok(value) => send_resp(Response::Ok(value))?,
                    Err(e) => send_resp(Response::Err(format!("{}", e)))?,
                },
                Request::Set { key, value } => match self.engine.set(key, value) {
                    Ok(_) => send_resp(Response::Ok(None))?,
                    Err(e) => send_resp(Response::Err(format!("{}", e)))?,
                },
                Request::Remove { key } => match self.engine.remove(key) {
                    Ok(_) => send_resp(Response::Ok(None))?,
                    Err(e) => send_resp(Response::Err(format!("{}", e)))?,
                },
            };
        }
        Ok(())
    }
}
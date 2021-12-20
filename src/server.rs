use crate::{KvsEngine, Result};
use crate::command::{Request, Response};
use serde_json::Deserializer;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use tracing::{debug, error};
use crate::thread_pool::{ThreadPool};

/// The server of a key value store.
/// It listens for incoming requests on a [`SocketAddr`] and then starts a thread for
/// each request. The threads will take a handle to the engine, and use that engine to process
/// the request.
pub struct KvsServer<E: KvsEngine, P: ThreadPool> {
    /// the storage engine being used
    engine: E,
    /// threads that will perform work using a handle to the engine
    pool: P,
}

impl<E: KvsEngine, P: ThreadPool> KvsServer<E, P> {
    /// Create a `KvsServer` with a given storage engine.
    pub fn new(engine: E, pool: P) -> Self {
       KvsServer {
            engine,
            pool,
        }
    }

    /// starts a server listening on the given address
    ///
    /// # Errors
    /// returns [`KvsError`] if the server could not be started
    pub fn run<A: ToSocketAddrs>(self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let eng = self.engine.clone();
                    self.pool.spawn(move || {
                        if let Err(e) = serve(eng, stream) {
                            error!("Error on serving client: {}", e);
                        }
                    });

                }
                Err(e) => error!("Connection failed: {}", e),
            }
        }
        Ok(())
    }
}

/// Listens for and serves a request coming from the given `tcp` stream.
/// This function will: deserializes the request, executes the request in the KvsEngine,
/// and return a response to the client on the `tcp` stream
fn serve<E: KvsEngine>(engine: E, tcp: TcpStream) -> Result<()> {
    let peer_addr = tcp.peer_addr()?;
    let stream_reader = BufReader::new(&tcp);
    let mut stream_writer = BufWriter::new(&tcp);
    let req_reader = Deserializer::from_reader(stream_reader).into_iter::<Request>();

    let mut send_resp = move |resp: Response| -> Result<()> {
        serde_json::to_writer(&mut stream_writer, &resp)?;
        stream_writer.flush()?;
        debug!("Response sent to {}: {:?}", peer_addr, resp);
        Ok(())
    };

    for req in req_reader {
        let req = req?;
        debug!("Receive request from {}: {:?}", peer_addr, req);

        match req {
            Request::Get { key } => match engine.get(key) {
                Ok(value) => send_resp(Response::Ok(value))?,
                Err(e) => send_resp(Response::Err(format!("{}", e)))?,
            },
            Request::Set { key, value } => match engine.set(key, value) {
                Ok(_) => send_resp(Response::Ok(None))?,
                Err(e) => send_resp(Response::Err(format!("{}", e)))?,
            },
            Request::Remove { key } => match engine.remove(key) {
                Ok(_) => send_resp(Response::Ok(None))?,
                Err(e) => send_resp(Response::Err(format!("{}", e)))?,
            },
        };
    }
    Ok(())
}
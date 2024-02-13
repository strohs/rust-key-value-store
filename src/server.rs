use crate::{KvsEngine, Result};
use crate::command::{Request, Response};
use serde_json::Deserializer;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use tracing::{debug, error};
use crate::thread_pool::{ThreadPool};

/// A TCP socket server implementation over a key value storage engine.
/// It listens for incoming [`Request`]s on a [`SocketAddr`](https://doc.rust-lang.org/std/net/enum.SocketAddr.html),
/// deserializes the request, and then process the request on a new thread.
///
/// Each thread receives a handle to a [`KvsEngine`], and use that engine to process the request.
///
/// # Example
/// Create and run a new server listening on "127.0.0.1:4000", with 4 threads running on a Rayon
/// Thread Pool, using the KvStore storage engine
/// ```rust
/// use std::net::SocketAddr;
/// use std::path::Path;
/// use kvs::{KvStore, KvsServer, KvsEngine};
/// use kvs::thread_pool::{RayonThreadPool, ThreadPool};
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let addr: SocketAddr = "127.0.0.1:4000".parse()?; // the IP address and port the server will listen on
/// let pool = RayonThreadPool::new(4)?; // create a rayon thread pool with 4 threads
/// let engine = KvStore::open(Path::new("."))?;  // create a kv-store that will persist data in the current directory
/// // now create the server using the kvs engine and thread pool
/// let server = KvsServer::new(engine, pool);
/// // start the server
/// //server.run(addr)?;
/// #
/// # Ok(())
/// # }
/// ```
///
/// [`Request`]: ./enum.Request.html
///
pub struct KvsServer<E: KvsEngine, P: ThreadPool> {
    /// the kvs engine to use
    engine: E,
    /// a pool of threads that will perform work using a handle to the engine
    pool: P,
}

impl<E: KvsEngine, P: ThreadPool> KvsServer<E, P> {
    /// Create a new `KvsServer` using the given [`KvsEngine`] and [`ThreadPool`] implementation.
    pub fn new(engine: E, pool: P) -> Self {
       KvsServer {
            engine,
            pool,
        }
    }

    /// starts a server listening on the given address.
    /// Each request that comes in gets serviced on its own thread from the ThreadPool
    ///
    /// # Errors
    /// returns [`KvsError`] if the server could not be started
    ///
    /// [`KvsError`]: ./enum.KvsError.html
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

/// Listens for and processes kvs [`Request`]s coming over the given `tcp` stream
/// This function will: deserialize the request, execute the request in the KvsEngine,
/// and finally return a [`Response`] to the client on the `tcp` stream
///
/// [`Request`]: ./enum.Request.html
/// [`Response`]: ./enum.Response.html
///
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
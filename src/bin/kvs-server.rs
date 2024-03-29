//! The `kvs-server` executable.
//!
//! It supports the following command line arguments:
///
/// - `kvs-server [--addr IP-PORT] [--engine ENGINE-NAME]`
///
///   Start the server and begin listening for incoming connections. `--addr`
///   accepts an IP address, either v4 or v6, and a port number, with the format
///   `IP:PORT`. If `--addr` is not specified then listen on `127.0.0.1:4000`.
///
///   If `--engine` is specified, then `ENGINE-NAME` must be "kvs". Future versions
///   of the server will support the "sled" engine, but it has not yet been fully integrated.
///   If this is the first run (there is no data previously persisted) then the default
///   value is "kvs". If there is previously persisted data then the default is the
///   engine already in use. If data was previously persisted with a different
///   engine than selected, print an error and exit with a non-zero exit code.
///
///   Print an error and return a non-zero exit code on failure to bind a socket, if
///   `ENGINE-NAME` is invalid, if `IP-PORT` does not parse as an address.
///
/// - `kvs-server -V`
///
///   Print the version.

use std::env::current_dir;
use std::fs;
use std::net::SocketAddr;
use clap::{crate_version, App, Arg, arg_enum, value_t};
use kvs::{KvsEngine, KvsError, KvStore, Result, KvsServer, ThreadPool, RayonThreadPool};
use tracing::{warn, info, Level};
use tracing_subscriber::{FmtSubscriber};
use std::process::exit;

arg_enum! {
    #[allow(non_camel_case_types)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Engine {
        kvs,
        sled
    }
}

// default values for the server command line
const DEFAULT_ADDRESS: &str = "127.0.0.1:4000";
const DEFAULT_ENGINE: Engine = Engine::kvs;
const DEFAULT_ENGINE_FILE: &str = "engine";


/// ['Opt'] holds parsed and validated options from the command line
#[derive(Debug)]
struct Opt {
    addr: SocketAddr,
    engine: Engine,
}

impl Opt {
    fn new(addr: SocketAddr, engine: Engine) -> Self {
        Self { addr, engine }
    }

    /// validates the `addr` and `requested_engine` parameters
    /// returns `Ok<Opt>` if everything is valid
    /// # Errors
    /// returns [`KvsError::Parsing`] if one of the parameters is invalid
    ///
    fn build(addr: &str, req_engine: Engine) -> Result<Opt> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|_| KvsError::Parsing(format!("could not parse {} into an IP addess and port", &addr)))?;

        // the requested engine parameter, if present, must be the same as the engine currently in use
        let engine = match current_engine()? {
            None => req_engine, // no current engine, use the requested engine
            Some(cur_engine) if req_engine == cur_engine => cur_engine, // current engine is the same as the requested engine
            // current engine != requested engine
            Some(cur_engine) => return Err(KvsError::Parsing(format!("the requested engine: {} does not match the engine currently in use: {}", req_engine, cur_engine)))
        };

        Ok(Opt::new(addr, engine))
    }
}


fn main() {
    // set up a tracing subscriber to log to STDERR
    subscriber_config();

    // parse command line arguments using clap
    let matches = App::new("kvs-server")
        .version(crate_version!())
        .author("strohs <strohs1@gmail.com>")
        .about("a multi-threaded key-value store")
        .arg(Arg::with_name("addr")
            .long("addr")
            .value_name("IP_ADDR:PORT")
            .help("sets the IP_ADDR:PORT that the server listens on")
            .default_value(DEFAULT_ADDRESS))
        .arg(Arg::with_name("engine")
            .long("engine")
            .value_name("ENGINE_NAME")
            .help("sets the storage engine to use, currently only 'kvs' is supported")
            .default_value("kvs"))
        .get_matches();

    // validate command line options, store them in Opt
    let addr = matches.value_of("addr").unwrap();
    // requested engine
    let req_engine: Engine = value_t!(matches, "engine", Engine).ok().unwrap_or(DEFAULT_ENGINE);
    let opt = match Opt::build(addr, req_engine) {
        Ok(opt) => opt,
        Err(err) => {
            eprintln!("{:?}", err);
            exit(1);
        }
    };

    // start the server
    if let Err(e) = run(opt) {
        eprintln!("{:?}", e);
        exit(1);
    }
}

/// starts a kvs server with the given `opt`ions
fn run(opt: Opt) -> Result<()> {
    info!("kvs-server {}", env!("CARGO_PKG_VERSION"));
    info!("Storage engine: {}", opt.engine);
    info!("Listening on {}", opt.addr);

    // write engine to engine file
    fs::write(current_dir()?.join("engine"), format!("{}", opt.engine))?;

    match opt.engine {
        Engine::kvs => run_with_engine(KvStore::open(&current_dir()?)?, opt.addr),
        Engine::sled => panic!("sled not currently implemented"),
        //Engine::sled => run_with_engine(SledKvsEngine::new(sled::open(current_dir()?)?), opt.addr),
    }
}


fn run_with_engine<E: KvsEngine>(engine: E, addr: SocketAddr) -> Result<()> {
    // created a thread pool with 4 threads, backed by a shared channel
    let pool = RayonThreadPool::new(4).unwrap();
    let server = KvsServer::new(engine, pool);
    server.run(addr)
}

/// determines if an "engine" file exists in the current directory and if so, returns a
/// ['Engine'] variant based on the string value within the engine file.
///
/// returns `Ok(None)` if an "engine" file does not (yet) exist,
/// returns `Some(Engine)` if the engine file exists and was parsed successfully
/// # Errors
/// returns ['KvsError'] if the engine file contains invalid string data
///
fn current_engine() -> Result<Option<Engine>> {
    let engine = current_dir()?.join(DEFAULT_ENGINE_FILE);
    if !engine.exists() {
        return Ok(None);
    }

    match fs::read_to_string(engine)?.parse() {
        Ok(engine) => Ok(Some(engine)),
        Err(e) => {
            // file is corrupted or invalid contents
            warn!("The content of the engine file is invalid: {}", e);
            Ok(None)
        }
    }
}

/// configures a tracing subscriber that will log to STDERR
fn subscriber_config() {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // log to stderr instead of stdout
        .with_writer(std::io::stderr)
        // completes the builder.
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting tracing default subscriber failed");
}
//! this binary starts the kvs server
//! to see the list of commands, type: `kvs-server --help`

use std::env::current_dir;
use std::fs;
use std::net::SocketAddr;
use clap::{crate_version, App, Arg, arg_enum, value_t};
use kvs::{KvsEngine, SledKvsEngine, KvsError, KvStore, Result, KvsServer};
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

const DEFAULT_ADDRESS: &str = "127.0.0.1:4000";
const DEFAULT_ENGINE: Engine = Engine::kvs;
// the name, file stem, of the "engine" file
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
            Some(cur_engine) => return Err(KvsError::Parsing(format!("the requested engine: {} does not match the engine currently in use: {}", req_engine.to_string(), cur_engine.to_string())))
        };

        Ok(Opt::new(addr, engine))
    }
}


fn main() {
    // set up a tracing subscriber to log to STDERR
    subscriber_config();

    // parse command line args
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
            .help("sets the storage engine to use, either 'kvs' or 'sled'")
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


fn run(opt: Opt) -> Result<()> {
    info!("kvs-server {}", env!("CARGO_PKG_VERSION"));
    info!("Storage engine: {}", opt.engine);
    info!("Listening on {}", opt.addr);

    // write engine to engine file
    fs::write(current_dir()?.join("engine"), format!("{}", opt.engine))?;

    match opt.engine {
        Engine::kvs => run_with_engine(KvStore::open(&current_dir()?)?, opt.addr),
        Engine::sled => run_with_engine(SledKvsEngine::new(sled::open(current_dir()?)?), opt.addr),
    }
}

fn run_with_engine<E: KvsEngine>(engine: E, addr: SocketAddr) -> Result<()> {
    let server = KvsServer::new(engine);
    server.run(addr)
}

/// determines if there is an "engine" file in use and returns the value of that file, else None
///
/// returns `Ok(None)` if an "engine" file does not (yet) exist, `Some(Engine)`
/// if the engine file exists and was parsed successfully
/// Errors if the engine file contains invalid data
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
        // log to stderr instrad of stdout
        .with_writer(std::io::stderr)
        // completes the builder.
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting tracing default subscriber failed");
}
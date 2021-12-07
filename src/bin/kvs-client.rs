//! The kvs-client executable supports the following command line arguments:
//!
//! `kvs-client set <KEY> <VALUE> [--addr IP-PORT]`
//!
//!     Set the value of a string key to a string.
//!     --addr accepts an IP address, either v4 or v6, and a port number, with the format IP:PORT. If --addr is not specified then connect on 127.0.0.1:4000.
//!     Print an error and return a non-zero exit code on server error, or if IP-PORT does not parse as an address.
//!
//! `kvs-client get <KEY> [--addr IP-PORT]`
//!
//!     Get the string value of a given string key.
//!     --addr accepts an IP address, either v4 or v6, and a port number, with the format IP:PORT. If --addr is not specified then connect on 127.0.0.1:4000.
//!     Print an error and return a non-zero exit code on server error, or if IP-PORT does not parse as an address.
//!
//! `kvs-client rm <KEY> [--addr IP-PORT]`
//!
//!     Remove a given string key.
//!     --addr accepts an IP address, either v4 or v6, and a port number, with the format IP:PORT. If --addr is not specified then connect on 127.0.0.1:4000.
//!     Print an error and return a non-zero exit code on server error, or if IP-PORT does not parse as an address. A "key not found" is also treated as an error in the "rm" command.
//!
//! `kvs-client -V`
//!
//!     Print the version.


use std::net::SocketAddr;
use clap::{crate_version, App, Arg, SubCommand, ArgMatches};
use kvs::{KvsClient, KvsError, Result, Request};
use tracing::{Level};
use tracing_subscriber::{FmtSubscriber};

const DEFAULT_ADDRESS: &str = "127.0.0.1:4000";

/// ['Opt'] holds parsed and validated options from the command line
#[derive(Debug)]
struct Opt {
    /// the server's ip:port
    addr: SocketAddr,
    req: Request,
}

impl Opt {
    fn new(addr: SocketAddr, req: Request) -> Self {
        Self { addr, req }
    }

    /// validates the `addr` parameter is a valid IP address and PORT
    /// returns `Ok<Opt>` if everything is valid
    /// # Errors
    /// returns [`KvsError::Parsing`] if one of the parameters is invalid
    ///
    fn build(addr: &str, req: Request) -> Result<Opt> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|_| KvsError::Parsing(format!("could not parse {} into an IP addess and port", &addr)))?;

        Ok(Opt::new(addr, req))
    }
}

fn main() -> Result<()> {
    // configure a subscriber that will log messages to STDERR
    subscriber_config();

    let matches = App::new("kvs-client")
        .version(crate_version!())
        .author("strohs <strohs1@gmail.com>")
        .about("a multi-threaded key-value store")
        .subcommands(vec![
            SubCommand::with_name("set")
                .about("Set the value of a string key to a string")
                .arg(Arg::with_name("KEY").required(true).index(1))
                .arg(Arg::with_name("VALUE").required(true).index(2)),
            SubCommand::with_name("get")
                .about("Get the string value of a given string key")
                .arg(Arg::with_name("KEY").required(true).index(1)),
            SubCommand::with_name("rm")
                .about("Removes a given key")
                .arg(Arg::with_name("KEY").required(true).index(1)),
        ])
        .arg(Arg::with_name("addr")
            .long("addr")
            .value_name("IP_ADDR:PORT")
            .help("sets the IP_ADDR:PORT of the server to connect to")
            .default_value(DEFAULT_ADDRESS))
        .get_matches();

    // parse commands into an Opt struct
    match parse_options(matches) {
        Ok(opt) => run(opt),
        Err(e) => Err(e),
    }
}

/// runs the specified request on the [`KvsClient`]
/// `opt` contains the server address and the request type to execute
fn run(opt: Opt) -> Result<()> {
    match opt.req {
        Request::Get { key } => {
            let mut client = KvsClient::connect(opt.addr)?;
            if let Some(value) = client.get(key)? {
                println!("{}", value);
            } else {
                println!("Key not found");
            }
        }
        Request::Set { key, value } => {
            let mut client = KvsClient::connect(opt.addr)?;
            client.set(key, value)?;
        }
        Request::Remove { key } => {
            let mut client = KvsClient::connect(opt.addr)?;
            client.remove(key)?;
        }
    }
    Ok(())
}

/// parses the matches from the command line into an [`Opt`] struct
fn parse_options(matches: ArgMatches) -> Result<Opt> {
    let addr = matches.value_of("addr").unwrap();
    match matches.subcommand() {
        ("set", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            let value = args.value_of("VALUE").map(String::from).unwrap();
            Opt::build(addr, Request::Set { key, value })
        }
        ("get", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            Opt::build(addr, Request::Get { key })
        }
        ("rm", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            Opt::build(addr, Request::Remove { key })
        }
        _ => panic!("unknown command received"),
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

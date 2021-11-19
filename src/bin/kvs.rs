use std::process::exit;
use clap::{crate_version, App, Arg, SubCommand};
use kvs::KvStore;

fn main() {
    let matches = App::new("kvs")
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
        .get_matches();

    // create a new kvStore in the current directory

    let mut kvs = match KvStore::open(".") {
        Err(e) => {
            eprintln!("{:#}", e);
            exit(1);
        },
        Ok(k) => k,
    };

    match matches.subcommand() {
        ("set", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            let value = args.value_of("VALUE").map(String::from).unwrap();
            if let Err(e) = kvs.set(key, value) {
                println!("{}", &e);
                exit(1);
            }
        }
        ("get", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            match kvs.get(key) {
                Err(e) => {
                    println!("{}", &e);
                    exit(1);
                },
                Ok(Some(value)) => {
                    println!("{}", value);
                    exit(0);
                },
                Ok(None) => {
                    println!("Key not found");
                    exit(0);
                }
            }
        }
        ("rm", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            if let Err(e) = kvs.remove(key) {
                println!("{}", &e);
                exit(1);
            }
        }
        _ => panic!("unknown command received"),
    }

}

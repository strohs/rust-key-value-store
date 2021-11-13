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

    let mut kvs = KvStore::new();

    match matches.subcommand() {
        ("set", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            let value = args.value_of("VALUE").map(String::from).unwrap();
            kvs.set(key, value);
        }
        ("get", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            kvs.get(key);
        }
        ("rm", Some(args)) => {
            let key = args.value_of("KEY").map(String::from).unwrap();
            kvs.remove(key);
        }
        _ => panic!("unkown command reveived"),
    }
}

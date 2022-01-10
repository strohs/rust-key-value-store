# Rust Key/Value Store

This is an implementation of a multi-threaded, persistent, key-value (kv) store that maps string keys to string values.
It is written in Rust and is part of the course [Practical Networked Applications in Rust](https://github.com/pingcap/talent-plan/blob/master/courses/rust/README.md)
It's great course designed to get users of Rust familiar with building basic networked applications, as well as 
Rust's concurrency and multi-threading libraries.


The key value store is implemented in a client/server architecture and supports three operations:
- "GET" a value from the store
- "SET" a key and value in the store
- "REMOVE" a key and value from the store

The [kv-storage engine](./src/engine/kvs.rs) keeps track of SET and REMOVE operations by persisting them to the local 
file system across a series of log files.
These log files are then used to rebuild the state of the store when it is (re)started. The store also implements
some compaction logic that cleans up stale data once it reaches a certain size (in bytes).

See the [module level documentation](./src/lib.rs) for more information.


## Prerequisites
You will need to have installed at least Rust version 1.56 as well as Cargo.

## Running
build the library and its client and server executables:
> cargo build

### start the kvs-server
- from a terminal, start the `kvs-server` (by default it will bind to localhost on port 4000)
    > ./target/debug/kvs-server


- OR you can provide your own address and port
    > ./target/debug/kvs-server IP-ADDRESS:PORT

the server will output debug information to the terminal as it is running. The log files will be written to the same
directory as you run the server from. They will begin with an integer and end in ".log"


### run the client
The client is used to send GET, SET and REMOVE operations to the server.
From a separate terminal window run the client:

- to set a key/value pair
    > ./target/debug/kvs-client set mykey myvalue


- to get the value associated with key: "mykey"
    > ./target/debug/kvs-client get mykey
    >
    > myvalue


- to remove a key/value pair
    > ./target/debug/kvs-client rm mykey


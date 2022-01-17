# Rust Key/Value Store

A multi-threaded, persistent, key-value (kv) store that maps string keys to string values, written in Rust.

It is part of my coursework from PingCap's [Practical Networked Applications in Rust](https://github.com/pingcap/talent-plan/blob/master/courses/rust/README.md)
The goal of this course is to teach new Rust programmers how to build real-world systems programs, with all the 
desirable Rust characteristics, including high-performance, reliability, and easy concurrency; and to do so using 
the best practices that might not be evident to newcomers.


The store supports three operations:
- "GET <key>" get a value from the store
- "SET <key> <value>" set a key and value in the store
- "RM <key>" remove a key and value from the store


This is a command line driven application.
It consists of separate [client](./src/bin/kvs-client.rs) and [server](./src/bin/kvs-server.rs) executables that use synchronous networking over a custom protocol
to send/receive data to/from the kvstore engine.

The client is mainly a "helper" application. You use it to send "GET", "SET", or "REMOVE" operations, one at a time, to a running server, and then print the result of the operation to the terminal.

The server implements the actual [storage engine](./src/engine/kvs.rs) logic. It stores the kv pairs in-memory, but also persists them to disk, so that they can be restored every time the server is re-started.


See the [module level documentation](./src/lib.rs) for more information.


## Prerequisites
You will need to have installed at least Rust version 1.56 as well as Cargo.

## Running
build the kvs library and its client and server executables:
> cargo build

### start the kvs-server
- from a terminal, start the `kvs-server` (by default it will bind to localhost on port 4000)
    > ./target/debug/kvs-server


- OR you can provide your own address and port
    > ./target/debug/kvs-server IP-ADDRESS:PORT

the server will output debug information to the terminal as it is running. The log files will be written to the same
directory as you run the server from. They will begin with an integer and end in ".log". As you set kv pairs in the store you
should be able to "cat" the .log file(s) and see them persisted there.


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

  
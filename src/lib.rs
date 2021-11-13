use std::process::exit;

pub struct KvStore {}

impl KvStore {
    pub fn new() -> Self {
        KvStore {}
    }

    pub fn get(&self, key: String) -> Option<String> {
        eprintln!("unimplemented!");
        exit(1);
    }

    pub fn set(&mut self, key: String, value: String) {
        eprintln!("unimplemented!");
        exit(1);
    }

    pub fn remove(&mut self, key: String) {
        eprintln!("unimplemented!");
        exit(1);
    }
}

use std::thread;
use crate::Result;
use super::ThreadPool;

/// a simple thread-pool that is not actually a pool. It starts a new thread on every spawn
/// request
#[allow(dead_code)]
pub struct NaiveThreadPool {
    threads: u32,
}

impl ThreadPool for NaiveThreadPool {

    fn new(threads: u32) -> Result<Self> {
        Ok(NaiveThreadPool {
            threads
        })
    }

    fn spawn<F>(&self, job: F) where F: FnOnce() + Send + 'static {
        // let hamdle = thread::Builder::new()
        //     .name("thread1".into_string())
        //     .spawn(job);
        thread::spawn(job);
    }
}

use std::thread;
use crossbeam::channel;
use crossbeam::channel::{Sender, Receiver};
use crate::{ThreadPool, Result};
use tracing::{error, debug, instrument};

/// A thread pool implemented with a shared job queue (i.e. channel).
///
/// This implementation uses the MPMC [`channel`] provided by the crossbeam crate.
/// Specifically, we are using it as a single producer, multiple consumer. The single producer
/// is this type itself, and the threads in the pool are the consumers
///
/// If a spawned task panics, the old thread will be destroyed and a new one will be
/// created. It fails silently when any failure to create the thread at the OS level
/// is captured after the thread pool is created. So, the thread number in the pool
/// can decrease to zero, then spawning a task to the thread pool will panic.
///
/// [`channel`]: https://docs.rs/crossbeam/0.8.1/crossbeam/channel/index.html
pub struct SharedQueueThreadPool {
    /// the sending part of the channel
    tx: Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl ThreadPool for SharedQueueThreadPool {

    /// create a new "thread pool" with the given number of `threads`.
    /// Every thread created will have a handle to the receiving end of the channel
    fn new(threads: u32) -> Result<Self> {
        let (tx, rx) = channel::unbounded::<Box<dyn FnOnce() + Send + 'static>>();
        for _ in 0..threads {
            let task_rx = TaskReceiver(rx.clone());
            thread::Builder::new().spawn(move || run_tasks(task_rx))?;
        }
        Ok(SharedQueueThreadPool { tx })
    }

    /// Spawns a function into the thread pool.
    ///
    /// # Panics
    ///
    /// Panics if the thread pool has no thread.
    fn spawn<F>(&self, job: F)
        where
            F: FnOnce() + Send + 'static,
    {
        self.tx
            .send(Box::new(job))
            .expect("There are no threads in the pool");
    }
}

/// A type that can receive tasks (i.e. closures) from a channel and run them.
/// Additionally, this type is responsible for restarting any threads that panicked
#[derive(Clone, Debug)]
struct TaskReceiver(Receiver<Box<dyn FnOnce() + Send + 'static>>);

impl Drop for TaskReceiver {
    #[instrument]
    fn drop(&mut self) {
        debug!("dropping thread");
        if thread::panicking() {
            debug!("thread panicked, starting a new thread");
            let task_rx = self.clone();
            if let Err(e) = thread::Builder::new().spawn(move || run_tasks(task_rx)) {
                error!("Failed to spawn a thread: {}", e);
            }
        }
    }
}

/// this function waits for a task to arrive on its (wrapped) receiver, and then runs the task
#[instrument]
fn run_tasks(rx: TaskReceiver) {
    loop {
        match rx.0.recv() {
            Ok(task) => {
                debug!("received a new task");
                task();
            }
            Err(_) => debug!("Thread exited because the thread pool was destroyed."),
        }
    }
}
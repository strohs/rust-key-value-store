//! Various thread pool implementations
//! - [`RayonThreadPool`] a work stealing thread pool that uses the Rayon library
//! - [`SharedQueueThreadPool`] a thread pool implemented using a shared queue
//!
use crate::Result;

/// A pool of threads
pub trait ThreadPool {

    /// Creates a new thread pool, immediately spawning the specified number of threads.
    /// Returns an error if any thread fails to spawn. All previously-spawned threads are terminated.
    fn new(threads: u32) -> Result<Self>
        where
            Self: Sized;

    /// Spawn a function into the thread-pool.
    /// Spawning always succeeds, but if the function panics, the thread-pool continues
    /// to operate with the same number of threads. The thread count is not reduced nor is
    /// the thread pool destroyed, corrupted or invalidated.
    fn spawn<F>(&self, job: F) where F: FnOnce() + Send + 'static;

}

mod naive;
mod shared_queue;
mod rayon_pool;

pub use self::naive::NaiveThreadPool;
pub use self::shared_queue::SharedQueueThreadPool;
pub use self::rayon_pool::RayonThreadPool;
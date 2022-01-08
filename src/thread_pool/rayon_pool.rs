use crate::{ThreadPool, Result, KvsError};
use tracing::{debug};
use rayon;

/// A thread pool that uses a work stealing strategy as implemented by the [`Rayon`] library.
///
/// [`Rayon`]: https://docs.rs/rayon/latest/rayon/index.html
pub struct RayonThreadPool {
    pool: rayon::ThreadPool,
}

impl ThreadPool for RayonThreadPool {

    fn new(threads: u32) -> Result<Self> where Self: Sized {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build()
            .map_err(|e|
                KvsError::StringErr(format!("could not build thread pool: {:?}", &e)))?;
        debug!("created thread pool with {} threads", &threads);

        Ok(
            Self { pool }
        )
    }

    fn spawn<F>(&self, job: F) where F: FnOnce() + Send + 'static {
        self.pool.install(job);
    }
}
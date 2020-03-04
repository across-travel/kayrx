use crate::fiber::{block_pool, Spawner, io, timer};
use crate::fiber::JoinHandle;
use std::future::Future;

/// Handle to the runtime
#[derive(Debug, Clone)]
pub struct Handle {
    pub(super) spawner: Spawner,

    /// Handles to the I/O drivers
    pub(super) io_handle: io::Handle,

    /// Handles to the timer drivers
    pub(super) timer_handle: timer::Handle,

    /// Source of `Instant::now()`
    pub(super) clock: timer::Clock,

    /// Blocking pool spawner
    pub(super) blocking_spawner: block_pool::Spawner,
}

impl Handle {
    /// Enter the runtime context
    pub fn enter<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.blocking_spawner.enter(|| {
            let _io = io::set_default(&self.io_handle);

            timer::with_default(&self.timer_handle, &self.clock, || self.spawner.enter(f))
        })
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawner.spawn(future)
    }
}

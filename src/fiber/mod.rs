//! The runtime implementation that runs everything on the current thread.

pub(crate) mod inner;
pub(crate) mod block_pool;
mod arbiter;
mod builder;
mod context;
mod enter;
mod handle;
mod local;
mod runtime;
mod scheduler;
mod spawner;
mod system;
mod io;
mod timer;

pub use self::arbiter::Arbiter;
pub use self::builder::{Builder, SystemRunner};
pub use self::runtime::Runtime;
pub use self::system::System;

pub(crate) use handle::Handle;
pub(crate) use local::spawn_local;
pub(crate) use inner::JoinHandle;
pub(crate) use builder::BuilderInner;
pub(crate) use runtime::RuntimeInner;
use scheduler::BasicScheduler;
use block_pool::BlockingPool;
use enter::enter;
use local::LocalSet;
use spawner::Spawner;

use std::future::Future;

/// Spawns a future on the current arbiter.
///
/// # Panics
///
/// This function panics if  system is not running.
pub fn spawn<F>(future: F)
where
    F: futures_core::Future<Output = ()> + 'static,
{
    if !System::is_set() {
        panic!("System is not running");
    }

    Arbiter::spawn(future);
}

/// Take fiber to  global  runtime executor.
pub fn take<T>(fiber: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    context::spawn(fiber)
}

/// Run fiber  on the Threadpool.
pub fn run<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    block_pool::spawn_blocking(f)
}

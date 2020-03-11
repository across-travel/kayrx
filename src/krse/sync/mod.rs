#![cfg_attr(loom, allow(dead_code, unreachable_pub, unused_imports))]

//! Future-aware synchronization
//!
//! This module is enabled with the **`sync`** feature flag.
//!
//! Tasks sometimes need to communicate with each other. This module contains
//! basic abstractions for doing so:
//!
//! - [oneshot](oneshot/index.html), a way of sending a single value
//!   from one task to another.
//! - [mpsc](mpsc/index.html), a multi-producer, single-consumer channel for
//!   sending values between tasks.
//! - [`Mutex`](struct.Mutex.html), an asynchronous `Mutex`-like type.
//! - [watch](watch/index.html), a single-producer, multi-consumer channel that
//!   only stores the **most recently** sent value.

    
pub mod local;
pub mod broadcast;
pub mod mpsc;
pub mod oneshot; 
pub mod watch;
pub(crate) mod atomic;

mod barrier;
mod mutex;
mod notify;
mod rwlock;
mod semaphore;

pub use notify::Notify;
pub use barrier::{Barrier, BarrierWaitResult};
pub use mutex::{Mutex, MutexGuard};
pub use semaphore::{Semaphore, SemaphorePermit};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};

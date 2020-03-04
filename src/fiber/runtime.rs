use std::future::Future;
use std::io;

use crate::fiber::{Handle, LocalSet, BuilderInner, JoinHandle, timer, BasicScheduler, BlockingPool};
use crate::krse::thread::ParkThread;

/// Single-threaded runtime provides a way to start reactor
/// and runtime on the current thread.
///
/// See [module level][mod] documentation for more details.
///
/// [mod]: index.html
#[derive(Debug)]
pub struct Runtime {
    local: LocalSet,
    rt: RuntimeInner,
}

impl Runtime {
    #[allow(clippy::new_ret_no_self)]
    /// Returns a new runtime initialized with default configuration values.
    pub fn new() -> io::Result<Runtime> {
        let rt = BuilderInner::new()
                .enable_io()
                .enable_timer()
                .build()?;

        Ok(Runtime {
            rt,
            local: LocalSet::new(),
        })
    }

    /// Spawn a future onto the single-threaded runtime.
    ///
    /// See [module level][mod] documentation for more details.
    ///
    /// [mod]: index.html
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use futures::{future, Future, Stream};
    /// use kayrx::fiber::Runtime;
    ///
    /// # fn dox() {
    /// // Create the runtime
    /// let mut rt = Runtime::new().unwrap();
    ///
    /// // Spawn a future onto the runtime
    /// rt.spawn(future::lazy(|_| {
    ///     println!("running on the runtime");
    /// }));
    /// # }
    /// # pub fn main() {}
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics if the spawn fails. Failure occurs if the executor
    /// is currently at capacity and is unable to spawn a new future.
    pub fn spawn<F>(&self, future: F) -> &Self
    where
        F: Future<Output = ()> + 'static,
    {
        self.local.spawn_local(future);
        self
    }

    /// Runs the provided future, blocking the current thread until the future
    /// completes.
    ///
    /// This function can be used to synchronously block the current thread
    /// until the provided `future` has resolved either successfully or with an
    /// error. The result of the future is then returned from this function
    /// call.
    ///
    /// Note that this function will **also** execute any spawned futures on the
    /// current thread, but will **not** block until these other spawned futures
    /// have completed. Once the function returns, any uncompleted futures
    /// remain pending in the `Runtime` instance. These futures will not run
    /// until `block_on` or `run` is called again.
    ///
    /// The caller is responsible for ensuring that other spawned futures
    /// complete execution by calling `block_on` or `run`.
    pub fn block_on<F>(&mut self, f: F) -> F::Output
    where
        F: Future + 'static,
    {
        let res = self.local.block_on(&mut self.rt, f);
        res
    }
}



#[derive(Debug)]
pub struct RuntimeInner {
    /// Fiber executor
    pub(crate) kind: Kind,

    /// Handle to runtime, also contains driver handles
    pub handle: Handle,

    /// Blocking pool handle, used to signal shutdown
    pub blocking_pool: BlockingPool,
}

/// The runtime executor is either a thread-pool or a current-thread executor.
#[derive(Debug)]
pub(crate)enum Kind {
    /// Execute all fibers on the current-thread.
    Basic(BasicScheduler<timer::Driver>),
}

/// After thread starts / before thread stops
pub(crate)type Callback = ::std::sync::Arc<dyn Fn() + Send + Sync>;

impl RuntimeInner {
    /// Create a new runtime instance with default configuration values.
    ///
    /// This results in a scheduler.
    ///
    pub fn new() -> io::Result<Self> {
        let ret = BuilderInner::new().enable_all().build();
        ret
    }

    /// Spawn a future onto the runtime.
    ///
    /// This spawns the given future onto the runtime's executor, then responsible 
    /// for polling the future until it completes.
    ///
    /// This function panics if the spawn fails. Failure occurs if the executor
    /// is currently at capacity and is unable to spawn a new future.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match &self.kind {
            Kind::Basic(exec) => exec.spawn(future),
        }
    }

    /// Run a future to completion on the runtime. This is the runtime's
    /// entry point.
    ///
    /// This runs the given future on the runtime, blocking until it is
    /// complete, and yielding its resolved result. Any fibers  which
    /// the future spawns internally will be executed on the runtime.
    ///
    /// This method should not be called from an asynchronous context.
    ///
    /// # Panics
    ///
    /// This function panics if the executor is at capacity, if the provided
    /// future panics, or if called within an asynchronous execution context.
    pub fn block_on<F: Future>(&mut self, future: F) -> F::Output {
        let kind = &mut self.kind;

        self.handle.enter(|| match kind {
            Kind::Basic(exec) => exec.block_on(future),
        })
    }

    /// Enter the runtime context
    pub fn enter<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.handle.enter(f)
    }

    /// Return a handle to the runtime's spawner.
    ///
    /// The returned handle can be used to spawn fibers that run on this runtime.
    ///
    pub fn handle(&self) -> &Handle {
        &self.handle
    }
}
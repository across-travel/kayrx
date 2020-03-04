use std::borrow::Cow;
use std::io;
use futures_channel::mpsc::unbounded;
use futures_channel::oneshot::{channel, Receiver};
use futures_util::future::{lazy, FutureExt};
use futures_core::Future;
use std::fmt;
use std::sync::Arc;

use crate::fiber::handle::Handle;
use crate::fiber::{block_pool, Spawner};
use crate::krse::thread::ParkThread;
use crate::fiber::arbiter::{Arbiter, SystemArbiter};
use crate::fiber::runtime::{Runtime, Callback, Kind, RuntimeInner};
use crate::fiber::system::System;
use crate::fiber::local::LocalSet;
use crate::fiber::BasicScheduler;
use crate::fiber::{io as io_in, timer};

/// Builder struct for a kayrx runtime.
///
/// Either use `Builder::build` to create a system and start fibers.
/// Alternatively, use `Builder::run` to start the kayrx runtime and
/// run a function in its context.
pub struct Builder {
    /// Name of the System. Defaults to "fiber" if unset.
    name: Cow<'static, str>,

    /// Whether the Arbiter will stop the whole System on uncaught panic. Defaults to false.
    stop_on_panic: bool,
}

impl Builder {
    pub(crate) fn new() -> Self {
        Builder {
            name: Cow::Borrowed("fiber"),
            stop_on_panic: false,
        }
    }

    /// Sets the name of the System.
    pub fn name<T: Into<String>>(mut self, name: T) -> Self {
        self.name = Cow::Owned(name.into());
        self
    }

    /// Sets the option 'stop_on_panic' which controls whether the System is stopped when an
    /// uncaught panic is thrown from a worker thread.
    ///
    /// Defaults to false.
    pub fn stop_on_panic(mut self, stop_on_panic: bool) -> Self {
        self.stop_on_panic = stop_on_panic;
        self
    }

    /// Create new System.
    ///
    /// This method panics if it can not create kayrx runtime
    pub fn build(self) -> SystemRunner {
        self.create_runtime(|| {})
    }

    /// Create new System that can run asynchronously.
    ///
    /// This method panics if it cannot start the system arbiter
    pub(crate) fn build_async(self, local: &LocalSet) -> AsyncSystemRunner {
        self.create_async_runtime(local)
    }

    /// This function will start kayrx runtime and will finish once the
    /// `System::stop()` message get called.
    /// Function `f` get called within kayrx runtime context.
    pub fn run<F>(self, f: F) -> io::Result<()>
    where
        F: FnOnce() + 'static,
    {
        self.create_runtime(f).run()
    }

    fn create_async_runtime(self, local: &LocalSet) -> AsyncSystemRunner {
        let (stop_tx, stop) = channel();
        let (sys_sender, sys_receiver) = unbounded();

        let system = System::construct(sys_sender, Arbiter::new_system(), self.stop_on_panic);

        // system arbiter
        let arb = SystemArbiter::new(stop_tx, sys_receiver);

        // start the system arbiter
        let _ = local.spawn_local(arb);

        AsyncSystemRunner { stop, system }
    }

    fn create_runtime<F>(self, f: F) -> SystemRunner
    where
        F: FnOnce() + 'static,
    {
        let (stop_tx, stop) = channel();
        let (sys_sender, sys_receiver) = unbounded();

        let system = System::construct(sys_sender, Arbiter::new_system(), self.stop_on_panic);

        // system arbiter
        let arb = SystemArbiter::new(stop_tx, sys_receiver);

        let mut rt = Runtime::new().unwrap();
        rt.spawn(arb);

        // init system arbiter and run configuration method
        rt.block_on(lazy(move |_| f()));

        SystemRunner { rt, stop, system }
    }
}

#[derive(Debug)]
pub(crate) struct AsyncSystemRunner {
    stop: Receiver<i32>,
    system: System,
}

impl AsyncSystemRunner {
    /// This function will start event loop and returns a future that
    /// resolves once the `System::stop()` function is called.
    pub(crate) fn run_nonblocking(self) -> impl Future<Output = Result<(), io::Error>> + Send {
        let AsyncSystemRunner { stop, .. } = self;

        // run loop
        lazy(|_| {
            Arbiter::run_system(None);
            async {
                let res = match stop.await {
                    Ok(code) => {
                        if code != 0 {
                            Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!("Non-zero exit code: {}", code),
                            ))
                        } else {
                            Ok(())
                        }
                    }
                    Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
                };
                Arbiter::stop_system();
                return res;
            }
        })
        .flatten()
    }
}

/// Helper object that runs System's event loop
#[must_use = "SystemRunner must be run"]
#[derive(Debug)]
pub struct SystemRunner {
    rt: Runtime,
    stop: Receiver<i32>,
    system: System,
}

impl SystemRunner {
    /// This function will start event loop and will finish once the
    /// `System::stop()` function is called.
    pub fn run(self) -> io::Result<()> {
        let SystemRunner { mut rt, stop, .. } = self;

        // run loop
        Arbiter::run_system(Some(&rt));
        let result = match rt.block_on(stop) {
            Ok(code) => {
                if code != 0 {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Non-zero exit code: {}", code),
                    ))
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        };
        Arbiter::stop_system();
        result
    }

    /// Execute a future and wait for result.
    pub fn block_on<F, O>(&mut self, fut: F) -> O
    where
        F: Future<Output = O> + 'static,
    {
        Arbiter::run_system(Some(&self.rt));
        let res = self.rt.block_on(fut);
        Arbiter::stop_system();
        res
    }
}

pub struct BuilderInner {

    /// Whether or not to enable the I/O driver
    enable_io: bool,

    /// Whether or not to enable the time driver
    enable_timer: bool,

    /// The number of worker threads, used by Runtime.
    ///
    /// Only used when not using the current-thread executor.
    core_threads: usize,

    /// Cap on thread usage.
    max_threads: usize,

    /// Name used for threads spawned by the runtime.
    pub thread_name: String,

    /// Stack size used for threads spawned by the runtime.
    pub thread_stack_size: Option<usize>,

    /// Callback to run after each thread starts.
    pub after_start: Option<Callback>,

    /// To run before each worker thread stops
    pub before_stop: Option<Callback>,
}

impl BuilderInner {
    /// Returns a new runtime builder initialized with default configuration
    /// values.
    ///
    /// Configuration methods can be chained on the return value.
    pub fn new() -> BuilderInner {
        BuilderInner {

            // I/O defaults to "off"
            enable_io: false,

            // Time defaults to "off"
            enable_timer: false,

            // Default to use an equal number of threads to number of CPU cores
            core_threads: usize::max(1, num_cpus::get_physical()),

            max_threads: 512,

            // Default thread name
            thread_name: "kayrx-zone-worker".into(),

            // Do not set a stack size by default
            thread_stack_size: None,

            // No worker thread callbacks
            after_start: None,
            before_stop: None,
        }
    }

    pub fn enable_all(&mut self) -> &mut Self {
        self.enable_io();
        self.enable_timer();

        self
    }

    pub fn enable_io(&mut self) -> &mut Self {
        self.enable_io = true;
        self
    }

    pub fn enable_timer(&mut self) -> &mut Self {
        self.enable_timer = true;
        self
    }

    pub fn core_threads(&mut self, val: usize) -> &mut Self {
        assert_ne!(val, 0, "Core threads cannot be zero");
        self.core_threads = val;
        self
    }

    pub fn max_threads(&mut self, val: usize) -> &mut Self {
        assert_ne!(val, 0, "Thread limit cannot be zero");
        self.max_threads = val;
        self
    }

    pub fn thread_name(&mut self, val: impl Into<String>) -> &mut Self {
        self.thread_name = val.into();
        self
    }

    pub fn thread_stack_size(&mut self, val: usize) -> &mut Self {
        self.thread_stack_size = Some(val);
        self
    }

    pub fn on_thread_start<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.after_start = Some(Arc::new(f));
        self
    }

    pub fn on_thread_stop<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.before_stop = Some(Arc::new(f));
        self
    }

    pub fn build(&mut self) -> io::Result<RuntimeInner> {
        self.build_basic_runtime()
    }

    fn build_basic_runtime(&mut self) -> io::Result<RuntimeInner> {

        let clock = timer::create_clock();

        // Create I/O driver
        let (io_driver, io_handle) = io_in::create_driver(self.enable_io)?;

        let (driver, timer_handle) = timer::create_driver(self.enable_timer, io_driver, clock.clone());

        // And now put a single-threaded scheduler on top of the timer. When
        // there are no futures ready to do something, it'll let the timer or
        // the reactor to generate some new stimuli for the futures to continue
        // in their life.
        let scheduler = BasicScheduler::new(driver);
        let spawner = Spawner::Basic(scheduler.spawner());

        // Blocking pool
        let blocking_pool = block_pool::create_blocking_pool(self, &spawner, &io_handle, &timer_handle, &clock, self.max_threads);
        let blocking_spawner = blocking_pool.spawner().clone();

        Ok(RuntimeInner {
            kind: Kind::Basic(scheduler),
            handle: Handle {
                spawner,
                io_handle,
                timer_handle,
                clock,
                blocking_spawner,
            },
            blocking_pool,
        })
    }
}

impl Default for BuilderInner {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for BuilderInner {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Builder")
            .field("core_threads", &self.core_threads)
            .field("max_threads", &self.max_threads)
            .field("thread_name", &self.thread_name)
            .field("thread_stack_size", &self.thread_stack_size)
            .field("after_start", &self.after_start.as_ref().map(|_| "..."))
            .field("before_stop", &self.after_start.as_ref().map(|_| "..."))
            .finish()
    }
}

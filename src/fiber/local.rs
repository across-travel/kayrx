//! Runs `!Send` futures on the current thread.
use crate::fiber::inner::{self as fiber, queue::MpscQueues, JoinHandle, Schedule, Fiber};
use crate::fiber::runtime::RuntimeInner;
use crate::krse::task::AtomicWaker;

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

 #[derive(Debug)]
 pub struct LocalSet {
     scheduler: Rc<Scheduler>,
 }

#[derive(Debug)]
struct Scheduler {
    tick: Cell<u8>,

    queues: MpscQueues<Self>,

    /// Used to notify the `LocalFuture` when a task in the local task set is
    /// notified.
    waker: AtomicWaker,
}

pin_project! {
    struct LocalFuture<F> {
        scheduler: Rc<Scheduler>,
        #[pin]
        future: F,
    }
}

thread_local! {
    static CURRENT_TASK_SET: Cell<Option<NonNull<Scheduler>>> = Cell::new(None);
}

pub fn spawn_local<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
        CURRENT_TASK_SET.with(|current| {
            let current = current
                .get()
                .expect("`spawn_local` called from outside of a task::LocalSet!");
            let (task, handle) = fiber::joinable_local(future);
            unsafe {
                // safety: this function is unsafe to call outside of the local
                // thread. Since the call above to get the current task set
                // would not succeed if we were outside of a local set, this is
                // safe.
                current.as_ref().queues.push_local(task);
            }

            handle
        })
}

/// Max number of tasks to poll per tick.
const MAX_TASKS_PER_TICK: usize = 61;

impl LocalSet {
    /// Returns a new local task set.
    pub fn new() -> Self {
        Self {
            scheduler: Rc::new(Scheduler::new()),
        }
    }

    pub fn spawn_local<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let (task, handle) = fiber::joinable_local(future);
        unsafe {
            // safety: since `LocalSet` is not Send or Sync, this is
            // always being called from the local thread.
            self.scheduler.queues.push_local(task);
        }
        handle
    }

    pub(crate) fn block_on<F>(&self, rt: &mut RuntimeInner, future: F) -> F::Output
    where
        F: Future,
    {
        let scheduler = self.scheduler.clone();
        self.scheduler
            .with(move || rt.block_on(LocalFuture { scheduler, future }))
    }
}

impl Default for LocalSet {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Future> Future for LocalFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let scheduler = this.scheduler;
        let mut future = this.future;
        scheduler.waker.register_by_ref(cx.waker());

        if let Poll::Ready(output) = future.as_mut().poll(cx) {
            return Poll::Ready(output);
        }

        if scheduler.tick() {
            // If `tick` returns true, we need to notify the local future again:
            // there are still tasks remaining in the run queue.
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }
}

// === impl Scheduler ===

impl Schedule for Scheduler {
    fn bind(&self, task: &Fiber<Self>) {
        assert!(self.is_current());
        unsafe {
            self.queues.add_task(task);
        }
    }

    fn release(&self, task: Fiber<Self>) {
        // This will be called when dropping the local runtime.
        self.queues.release_remote(task);
    }

    fn release_local(&self, task: &Fiber<Self>) {
        debug_assert!(self.is_current());
        unsafe {
            self.queues.release_local(task);
        }
    }

    fn schedule(&self, task: Fiber<Self>) {
        if self.is_current() {
            unsafe { self.queues.push_local(task) };
        } else {
            let mut lock = self.queues.remote();
            lock.schedule(task, false);

            self.waker.wake();

            drop(lock);
        }
    }
}

impl Scheduler {
    fn new() -> Self {
        Self {
            tick: Cell::new(0),
            queues: MpscQueues::new(),
            waker: AtomicWaker::new(),
        }
    }

    fn with<F>(&self, f: impl FnOnce() -> F) -> F {
        struct Entered<'a> {
            current: &'a Cell<Option<NonNull<Scheduler>>>,
        }

        impl<'a> Drop for Entered<'a> {
            fn drop(&mut self) {
                self.current.set(None);
            }
        }

        CURRENT_TASK_SET.with(|current| {
            let prev = current.replace(Some(NonNull::from(self)));
            assert!(prev.is_none(), "nested call to local::Scheduler::with");
            let _entered = Entered { current };
            f()
        })
    }

    fn is_current(&self) -> bool {
        CURRENT_TASK_SET
            .try_with(|current| {
                current
                    .get()
                    .iter()
                    .any(|current| ptr::eq(current.as_ptr(), self as *const _))
            })
            .unwrap_or(false)
    }

    /// Tick the scheduler, returning whether the local future needs to be
    /// notified again.
    fn tick(&self) -> bool {
        assert!(self.is_current());
        for _ in 0..MAX_TASKS_PER_TICK {
            let tick = self.tick.get().wrapping_add(1);
            self.tick.set(tick);

            let task = match unsafe {
                // safety: we must be on the local thread to call this. The assertion
                // the top of this method ensures that `tick` is only called locally.
                self.queues.next_task(tick)
            } {
                Some(task) => task,
                // We have fully drained the queue of notified tasks, so the
                // local future doesn't need to be notified again — it can wait
                // until something else wakes a task in the local set.
                None => return false,
            };

            if let Some(task) = task.run(&mut || Some(self.into())) {
                unsafe {
                    // safety: we must be on the local thread to call this. The
                    // the top of this method ensures that `tick` is only called locally.
                    self.queues.push_local(task);
                }
            }
        }

        true
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        unsafe {
            // safety: these functions are unsafe to call outside of the local
            // thread. Since the `Scheduler` type is not `Send` or `Sync`, we
            // know it will be dropped only from the local thread.
            self.queues.shutdown();

            // Wait until all tasks have been released.
            // XXX: this is a busy loop, but we don't really have any way to park
            // the thread here?
            loop {
                self.queues.drain_pending_drop();
                self.queues.drain_queues();

                if !self.queues.has_tasks_remaining() {
                    break;
                }

                std::thread::yield_now();
            }
        }
    }
}

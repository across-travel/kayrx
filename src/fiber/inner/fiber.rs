use crate::krse::cell::CausalCell;
use crate::krse::alloc::Track;
use crate::fiber::inner::raw::{self, Vtable};
use crate::fiber::inner::state::State;
use crate::fiber::inner::waker::waker_ref;
use crate::fiber::inner::{RawFiber, JoinHandle, JoinError};

use std::cell::UnsafeCell;
use std::future::Future;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr::{self, NonNull};
use std::task::{Context, Poll, Waker};
use std::marker::PhantomData;
use std::{fmt, mem};


/// An owned handle to the task, tracked by ref count
pub(crate) struct Fiber<S: 'static> {
    raw: RawFiber,
    _p: PhantomData<S>,
}

unsafe impl<S: ScheduleSend + 'static> Send for Fiber<S> {}

/// Fiber result sent back
pub(crate) type Result<T> = std::result::Result<T, JoinError>;

pub(crate) trait Schedule: Sized + 'static {
        /// Bind a task to the executor.
        ///
        /// Guaranteed to be called from the thread that called `poll` on the task.
        fn bind(&self, task: &Fiber<Self>);

        /// The task has completed work and is ready to be released. The scheduler
        /// is free to drop it whenever.
        fn release(&self, task: Fiber<Self>);

        /// The has been completed by the executor it was bound to.
        fn release_local(&self, task: &Fiber<Self>);

        /// Schedule the task
        fn schedule(&self, task: Fiber<Self>);
}

/// Marker trait indicating that a scheduler can only schedule tasks which
/// implement `Send`.
///
/// Schedulers that implement this trait may not schedule `!Send` futures. If
/// trait is implemented, the corresponding `Fiber` type will implement `Send`.
pub(crate) trait ScheduleSend: Schedule + Send + Sync {}


/// The fiber cell. Contains the components of the fiber.
///
/// It is critical for `Header` to be the first field as the task structure will
/// be referenced by both *mut Cell and *mut Header.
#[repr(C)]
pub(super) struct Cell<T: Future> {
    /// Hot task state data
    pub(super) header: Header,

    /// Either the future or output, depending on the execution stage.
    pub(super) core: Core<T>,

    /// Cold data
    pub(super) trailer: Trailer,
}

/// The core of the fiber.
///
/// Holds the future or output, depending on the stage of execution.
pub(super) struct Core<T: Future> {
    stage: Stage<T>,
}

/// Crate public as this is also needed by the pool.
#[repr(C)]
pub(crate) struct Header {
    /// Fiber state
    pub(super) state: State,

    /// Pointer to the executor owned by the task
    pub(super) executor: CausalCell<Option<NonNull<()>>>,

    /// Pointer to next task, used for misc task linked lists.
    pub(crate) queue_next: UnsafeCell<*const Header>,

    /// Pointer to the next task in the ownership list.
    pub(crate) owned_next: UnsafeCell<Option<NonNull<Header>>>,

    /// Pointer to the previous task in the ownership list.
    pub(crate) owned_prev: UnsafeCell<Option<NonNull<Header>>>,

    /// Table of function pointers for executing actions on the task.
    pub(super) vtable: &'static Vtable,

    /// Track the causality of the future.  this is unit.
    pub(super) future_causality: CausalCell<()>,
}

/// Cold data is stored after the future.
pub(super) struct Trailer {
    /// Consumer task waiting on completion of this task.
    pub(super) waker: CausalCell<MaybeUninit<Option<Waker>>>,
}

/// Either the future or the output.
enum Stage<T: Future> {
    Running(Track<T>),
    Finished(Track<super::Result<T::Output>>),
    Consumed,
}


impl<S: 'static> Fiber<S> {
        pub(crate) unsafe fn from_raw(ptr: NonNull<Header>) -> Fiber<S> {
            Fiber {
                raw: RawFiber::from_raw(ptr),
                _p: PhantomData,
            }
        }

        pub(crate) fn header(&self) -> &Header {
            self.raw.header()
        }

        pub(crate) fn into_raw(self) -> NonNull<Header> {
            let raw = self.raw.into_raw();
            mem::forget(self);
            raw
        }
}

impl<S: Schedule> Fiber<S> {
    /// Returns `self` when the task needs to be immediately re-scheduled
    pub(crate) fn run<F>(self, mut executor: F) -> Option<Self>
    where
        F: FnMut() -> Option<NonNull<S>>,
    {
        if unsafe {
            self.raw
                .poll(&mut || executor().map(|ptr| ptr.cast::<()>()))
        } {
            Some(self)
        } else {
            // Cleaning up the `Fiber` instance is done from within the poll
            // function.
            mem::forget(self);
            None
        }
    }
    /// Pre-emptively cancel the task as part of the shutdown process.
    pub(crate) fn shutdown(self) {
        self.raw.cancel_from_queue();
        mem::forget(self);
    }
}

/// Create a new task with an associated join handle
pub(crate) fn joinable<T, S>(task: T) -> (Fiber<S>, JoinHandle<T::Output>)
where
        T: Future + Send + 'static,
        S: ScheduleSend,
{
        let raw = RawFiber::new_joinable::<_, S>(task);

        let task = Fiber {
            raw,
            _p: PhantomData,
        };

        let join = JoinHandle::new(raw);

        (task, join)
}

/// Create a new `!Send` task with an associated join handle
pub(crate) fn joinable_local<T, S>(task: T) -> (Fiber<S>, JoinHandle<T::Output>)
where
            T: Future + 'static,
            S: Schedule,
{
            let raw = RawFiber::new_joinable_local::<_, S>(task);

            let task = Fiber {
                raw,
                _p: PhantomData,
            };

            let join = JoinHandle::new(raw);

            (task, join)
}

impl<T: Future> Cell<T> {
    /// Allocate a new task cell, containing the header, trailer, and core
    /// structures.
    pub(super) fn new<S>(future: T, state: State) -> Box<Cell<T>>
    where
        S: Schedule,
    {
        Box::new(Cell {
            header: Header {
                state,
                executor: CausalCell::new(None),
                queue_next: UnsafeCell::new(ptr::null()),
                owned_next: UnsafeCell::new(None),
                owned_prev: UnsafeCell::new(None),
                vtable: raw::vtable::<T, S>(),
                future_causality: CausalCell::new(()),
            },
            core: Core {
                stage: Stage::Running(Track::new(future)),
            },
            trailer: Trailer {
                waker: CausalCell::new(MaybeUninit::new(None)),
            },
        })
    }
}

impl<T: Future> Core<T> {
    pub(super) fn transition_to_consumed(&mut self) {
        self.stage = Stage::Consumed
    }

    pub(super) fn poll<S>(&mut self, header: &Header) -> Poll<T::Output>
    where
        S: Schedule,
    {
        let res = {
            let future = match &mut self.stage {
                Stage::Running(tracked) => tracked.get_mut(),
                _ => unreachable!("unexpected stage"),
            };

            // The future is pinned within the task. The above state transition
            // has ensured the safety of this action.
            let future = unsafe { Pin::new_unchecked(future) };

            // The waker passed into the `poll` function does not require a ref
            // count increment.
            let waker_ref = waker_ref::<T, S>(header);
            let mut cx = Context::from_waker(&*waker_ref);

            future.poll(&mut cx)
        };

        if res.is_ready() {
            self.stage = Stage::Consumed;
        }

        res
    }

    pub(super) fn store_output(&mut self, output: super::Result<T::Output>) {
        self.stage = Stage::Finished(Track::new(output));
    }

    pub(super) unsafe fn read_output(&mut self, dst: *mut Track<super::Result<T::Output>>) {
        use std::mem;

        dst.write(match mem::replace(&mut self.stage, Stage::Consumed) {
            Stage::Finished(output) => output,
            _ => unreachable!("unexpected state"),
        });
    }
}

impl Header {
    pub(super) fn executor(&self) -> Option<NonNull<()>> {
        unsafe { self.executor.with(|ptr| *ptr) }
    }
}


impl<S: 'static> Drop for Fiber<S> {
    fn drop(&mut self) {
        self.raw.drop_task();
    }
}

impl<S> fmt::Debug for Fiber<S> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Fiber").finish()
    }
}
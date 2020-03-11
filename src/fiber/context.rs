use crate::fiber::scheduler;
use crate::fiber::inner::JoinHandle;

use std::cell::Cell;
use std::future::Future;

#[derive(Clone, Copy)]
enum State {
    // default executor not defined
    Empty,

    // Basic scheduler (runs on the current-thread)
    Basic(*const scheduler::SchedulerPriv),
}

thread_local! {
    /// Thread-local tracking the current executor
    static EXECUTOR: Cell<State> = Cell::new(State::Empty)
}

// ===== global spawn fns =====

/// Spawns a future on the default executor.
pub(crate) fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    EXECUTOR.with(|current_executor| match current_executor.get() {
        State::Basic(basic_scheduler_ptr) => {
            let basic_scheduler = unsafe { &*basic_scheduler_ptr };

            // Safety: The `BasicScheduler` value set the thread-local (same
            // thread).
            unsafe { basic_scheduler.spawn(future) }
        }
        State::Empty => {
            // Explicit drop of `future` silences the warning that `future` is
            // not used when neither rt-* feature flags are enabled.
            drop(future);
            panic!("must be called from the context of runtime configured with `scheduler`");
        }
    })
}

pub(super) fn with_basic_scheduler<F, R>(
    basic_scheduler: &scheduler::SchedulerPriv,
    f: F,
) -> R
where
    F: FnOnce() -> R,
{
    with_state(
        State::Basic(basic_scheduler as *const scheduler::SchedulerPriv),
        f,
    )
}

fn with_state<F, R>(state: State, f: F) -> R
where
    F: FnOnce() -> R,
{
    EXECUTOR.with(|cell| {
        let was = cell.replace(State::Empty);

        // Ensure that the executor is removed from the thread-local context
        // when leaving the scope. This handles cases that involve panicking.
        struct Reset<'a>(&'a Cell<State>, State);

        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }

        let _reset = Reset(cell, was);

        cell.set(state);

        f()
    })
}

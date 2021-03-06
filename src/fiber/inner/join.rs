use crate::fiber::inner::RawFiber;
use crate::krse::alloc::Track;

use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::any::Any;
use std::io;
use std::sync::Mutex;

pub struct JoinHandle<T> {
    raw: Option<RawFiber>,
    _p: PhantomData<T>,
}

unsafe impl<T: Send> Send for JoinHandle<T> {}
unsafe impl<T: Send> Sync for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    pub(super) fn new(raw: RawFiber) -> JoinHandle<T> {
        JoinHandle {
            raw: Some(raw),
            _p: PhantomData,
        }
    }
}

impl<T> Unpin for JoinHandle<T> {}

impl<T> Future for JoinHandle<T> {
    type Output = super::Result<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use std::mem::MaybeUninit;

        // Raw should always be set
        let raw = self.raw.as_ref().unwrap();

        // Load the current task state
        let mut state = raw.header().state.load();

        debug_assert!(state.is_join_interested());

        if state.is_active() {
            state = if state.has_join_waker() {
                raw.swap_join_waker(cx.waker(), state)
            } else {
                raw.store_join_waker(cx.waker())
            };

            if state.is_active() {
                return Poll::Pending;
            }
        }

        let mut out = MaybeUninit::<Track<Self::Output>>::uninit();

        unsafe {
            // This could result in the task being freed.
            raw.read_output(out.as_mut_ptr() as *mut (), state);

            self.raw = None;

            Poll::Ready(out.assume_init().into_inner())
        }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        if let Some(raw) = self.raw.take() {
            if raw.header().state.drop_join_handle_fast() {
                return;
            }

            raw.drop_join_handle_slow();
        }
    }
}

impl<T> fmt::Debug for JoinHandle<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("JoinHandle").finish()
    }
}


pub struct JoinError {
    repr: Repr,
}

enum Repr {
    Cancelled,
    Panic(Mutex<Box<dyn Any + Send + 'static>>),
}

impl JoinError {
    pub(crate)  fn cancelled() -> JoinError {
        JoinError {
            repr: Repr::Cancelled,
        }
    }

    pub(crate) fn panic(err: Box<dyn Any + Send + 'static>) -> JoinError {
        JoinError {
            repr: Repr::Panic(Mutex::new(err)),
        }
    }

}

impl fmt::Display for JoinError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Cancelled => write!(fmt, "cancelled"),
            Repr::Panic(_) => write!(fmt, "panic"),
        }
    }
}

impl fmt::Debug for JoinError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Cancelled => write!(fmt, "JoinError::Cancelled"),
            Repr::Panic(_) => write!(fmt, "JoinError::Panic(...)"),
        }
    }
}

impl std::error::Error for JoinError {}

impl From<JoinError> for io::Error {
    fn from(src: JoinError) -> io::Error {
        io::Error::new(
            io::ErrorKind::Other,
            match src.repr {
                Repr::Cancelled => "task was cancelled",
                Repr::Panic(_) => "task panicked",
            },
        )
    }
}

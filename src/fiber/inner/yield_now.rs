use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Yields execution back to the runtime.
///
/// A fiber yields by awaiting on `yield_now()`, and may resume when that
/// future completes (with no output.) The current fiber will be re-added as
/// a pending fiber at the _back_ of the pending queue. Any other pending
/// fibers will be scheduled. No other waking is required for the fiber to
/// continue.
///
/// See also the usage example in the [fiber module](index.html#yield_now).
#[must_use = "yield_now does nothing unless polled/`await`-ed"]
pub async fn yield_now() {
        /// Yield implementation
        struct YieldNow {
            yielded: bool,
        }

        impl Future for YieldNow {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.yielded {
                    return Poll::Ready(());
                }

                self.yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }

        YieldNow { yielded: false }.await
}
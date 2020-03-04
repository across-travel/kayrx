//! Abstracts out the APIs necessary to `Runtime` for integrating the timer
//! driver. When the `timer` feature flag is **not** enabled. These APIs are
//! shells. This isolates the complexity of dealing with conditional
//! compilation.

use crate::krse::thread::Either;
use crate::fiber::io;
use crate::timer::{self, driver};

pub(crate) type Clock = timer::Clock;
pub(crate) type Driver = Either<driver::Driver<io::Driver>, io::Driver>;
pub(crate) type Handle = Option<driver::Handle>;

pub(crate) fn create_clock() -> Clock {
        Clock::new()
}

/// Create a new timerr driver / handle pair
pub(crate) fn create_driver(
        enable: bool,
        io_driver: io::Driver,
        clock: Clock,
) -> (Driver, Handle) {
        if enable {
            let driver = driver::Driver::new(io_driver, clock);
            let handle = driver.handle();

            (Either::A(driver), Some(handle))
        } else {
            (Either::B(io_driver), None)
        }
}

pub(crate) fn with_default<F, R>(handle: &Handle, clock: &Clock, f: F) -> R
where
        F: FnOnce() -> R,
{
        let _timer = handle.as_ref().map(|handle| driver::set_default(handle));
        clock.enter(f)
}

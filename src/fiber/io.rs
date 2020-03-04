//! Abstracts out the APIs necessary to `fiber` for integrating the I/O
//! driver. When the `time` feature flag is **not** enabled. These APIs are
//! shells. This isolates the complexity of dealing with conditional
//! compilation.

use crate::krse::io::driver;
use crate::krse::thread::{Either, ParkThread};
use std::io;

/// The driver value the fiber passes to the `timer` layer.
///
/// When the `io-driver` feature is enabled, this is the "real" I/O driver
/// backed by Mio. Without the `io-driver` feature, this is a thread parker
/// backed by a condition variable.
pub(crate) type Driver = Either<driver::Driver, ParkThread>;

/// The handle the fiber stores for future use.
///
/// When the `io-driver` feature is **not** enabled, this is `()`.
pub(crate) type Handle = Option<driver::Handle>;

pub(crate) fn create_driver(enable: bool) -> io::Result<(Driver, Handle)> {
        if enable {
            let driver = driver::Driver::new()?;
            let handle = driver.handle();

            Ok((Either::A(driver), Some(handle)))
        } else {
            let driver = ParkThread::new();
            Ok((Either::B(driver), None))
        }
}

pub(crate) fn set_default(handle: &Handle) -> Option<driver::DefaultGuard<'_>> {
        handle.as_ref().map(|handle| driver::set_default(handle))
}

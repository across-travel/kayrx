//! Asynchronous signal handling
//!
//! Note that signal handling is in general a very tricky topic and should be
//! used with great care. This crate attempts to implement 'best practice' for
//! signal handling, but it should be evaluated for your own applications' needs
//! to see if it's suitable.
//!
//! The are some fundamental limitations of this crate documented on the OS
//! specific structures, as well.
//!
//! # Examples
//!
//! Print on "ctrl-c" notification.
//!
//! ```rust,no_run
//! use kayrx::krse::signal;
//!
//! #[kayrx::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     signal::ctrl_c().await?;
//!     println!("ctrl-c received!");
//!     Ok(())
//! }
//! ```
//!
//! Wait for SIGHUP on Unix
//!
//! ```rust,no_run
//! use kayrx::krse::signal::unix::{signal, SignalKind};
//!
//! #[kayrx::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // An infinite stream of hangup signals.
//!     let mut stream = signal(SignalKind::hangup())?;
//!
//!     // Print whenever a HUP signal is received
//!     loop {
//!         stream.recv().await;
//!         println!("got signal HUP");
//!     }
//! }
//! ```

mod ctrl_c;
pub mod hook; 
pub mod unix;
mod registry;

mod os {
    pub(crate) use super::unix::{OsExtraData, OsStorage};
}

pub use ctrl_c::ctrl_c;

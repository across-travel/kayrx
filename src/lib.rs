#![allow(clippy::cognitive_complexity, warnings)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub
)]

#[macro_use]
extern crate log;

#[cfg(not(test))] 
pub use kayrx_macro::*;
pub mod codec;
pub mod fiber;
pub mod framed;
pub mod server;
pub mod krse;
pub mod service;
pub mod timer;
pub mod connect;
pub mod http;
pub mod router;
pub mod secure;
pub mod web;
pub mod websocket;
pub mod util;

pub use self::fiber::{spawn, take, run};

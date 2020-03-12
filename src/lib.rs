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
pub use kayrx_macro::main;
pub use kayrx_macro::{test, connect, delete, get, post, head, options, patch, put, trace};
pub mod codec;
pub mod connect;
pub mod fiber;
pub mod framed;
pub mod http;
pub mod krse;
pub mod router;
pub mod secure;
pub mod server;
pub mod service;
pub mod timer;
pub mod web;
pub mod websocket;
pub mod util;

pub use fiber::{spawn, take, run};

//! General Multi-thread Server

mod accept;
mod builder;
mod config;
mod server;
mod service;
mod signal;
mod socket;
mod worker;

pub use self::builder::ServerBuilder;
pub use self::config::{ServiceConfig, ServiceRuntime};
pub use self::server::Server;
pub use self::service::ServiceFactory;

#[doc(hidden)]
pub use self::socket::FromStream;

/// Socket id token
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Token(usize);

impl Token {
    pub(crate) fn next(&mut self) -> Token {
        let token = Token(self.0);
        self.0 += 1;
        token
    }
}

/// Start server building process
pub fn new() -> ServerBuilder {
    ServerBuilder::default()
}
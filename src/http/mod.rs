//! HTTP Primitives.

mod builder;
mod cloneable;
mod config;
mod extensions;
mod helpers;
mod httpcodes;
mod payload;
mod request;
mod service;
pub(crate) mod message;
pub(crate) mod response;

pub mod body;
pub mod client;
pub mod encoding;
pub mod error;
pub mod header;
pub mod httpmessage;
pub mod h1;
pub mod h2;
pub mod test;

pub use self::builder::HttpServiceBuilder;
pub use self::config::{KeepAlive, ServiceConfig};
pub use self::extensions::Extensions;
pub use self::message::{Message, RequestHead, RequestHeadType, ResponseHead};
pub use self::payload::{Payload, PayloadStream};
pub use self::request::Request;
pub use self::response::{Response, ResponseBuilder};
pub use self::service::HttpService;

/// Various HTTP related types

pub use http::header::{HeaderName, HeaderValue};
pub use http::uri::{self, PathAndQuery};
pub use http::{Uri, Method, StatusCode, Version};

pub use crate::http::header::HeaderMap;
pub use crate::http::message::ConnectionType;

pub(crate) use self::error::{ResponseError, Result};
pub(crate) use self::httpmessage::HttpMessage;
pub(crate) use self::test::TestBuffer;

/// Http protocol
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Protocol {
    Http1,
    Http2,
}

pub mod dev {
    pub use super::cloneable::CloneableService;
    pub use super::h1::dev::{DispatcherState, Flags};
}
//! HTTP Primitives.

mod builder;
mod cloneable;
mod config;
mod extensions;
mod helpers;
mod httpcodes;
pub(crate) mod message;
mod payload;
mod request;
pub(crate) mod response;
mod service;

pub mod body;
pub mod client;
pub mod encoding;
pub mod header;
pub mod httpmessage;
pub mod error;
pub mod h1;
pub mod h2;
pub mod test;

pub use builder::HttpServiceBuilder;
pub use config::{KeepAlive, ServiceConfig};
pub use error::{Error, ResponseError, Result};
pub use extensions::Extensions;
pub use httpmessage::HttpMessage;
pub use message::{Message, RequestHead, RequestHeadType, ResponseHead};
pub use payload::{Payload, PayloadStream};
pub use request::Request;
pub use response::{Response, ResponseBuilder};
pub use service::HttpService;
pub use test::TestBuffer;

/// Various HTTP related types

// re-exports
pub use http::header::{HeaderName, HeaderValue};
pub use http::uri::PathAndQuery;
pub use http::{uri, Error as HttpError, Uri};
pub use http::{Method, StatusCode, Version};

pub use crate::http::header::HeaderMap;
pub use crate::http::message::ConnectionType;

/// Http protocol
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Protocol {
    Http1,
    Http2,
}

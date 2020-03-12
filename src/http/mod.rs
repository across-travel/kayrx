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
pub use self::error::{Error, ResponseError, Result};
pub use self::extensions::Extensions;
pub use self::httpmessage::HttpMessage;
pub use self::message::{Message, RequestHead, RequestHeadType, ResponseHead};
pub use self::payload::{Payload, PayloadStream};
pub use self::request::Request;
pub use self::response::{Response, ResponseBuilder};
pub use self::service::HttpService;
pub use self::test::TestBuffer;

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

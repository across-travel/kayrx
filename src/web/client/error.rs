//! Http client errors
pub use crate::http::client::{
    ConnectError, FreezeRequestError, InvalidUrl, SendRequestError,
};
pub use crate::http::error::PayloadError;
pub use crate::http::error::HttpError;
pub use crate::websocket::HandshakeError as WsHandshakeError;
pub use crate::websocket::ProtocolError as WsProtocolError;

use crate::http::ResponseError;
use serde_json::error::Error as JsonError;

use crate::http::{header::HeaderValue, StatusCode};
use derive_more::{Display, From};

/// Websocket client error
#[derive(Debug, Display, From)]
pub enum WsClientError {
    /// Invalid response status
    #[display(fmt = "Invalid response status")]
    InvalidResponseStatus(StatusCode),
    /// Invalid upgrade header
    #[display(fmt = "Invalid upgrade header")]
    InvalidUpgradeHeader,
    /// Invalid connection header
    #[display(fmt = "Invalid connection header")]
    InvalidConnectionHeader(HeaderValue),
    /// Missing CONNECTION header
    #[display(fmt = "Missing CONNECTION header")]
    MissingConnectionHeader,
    /// Missing SEC-WEBSOCKET-ACCEPT header
    #[display(fmt = "Missing SEC-WEBSOCKET-ACCEPT header")]
    MissingWebSocketAcceptHeader,
    /// Invalid challenge response
    #[display(fmt = "Invalid challenge response")]
    InvalidChallengeResponse(String, HeaderValue),
    /// Protocol error
    #[display(fmt = "{}", _0)]
    Protocol(WsProtocolError),
    /// Send request error
    #[display(fmt = "{}", _0)]
    SendRequest(SendRequestError),
}

impl From<InvalidUrl> for WsClientError {
    fn from(err: InvalidUrl) -> Self {
        WsClientError::SendRequest(err.into())
    }
}

impl From<HttpError> for WsClientError {
    fn from(err: HttpError) -> Self {
        WsClientError::SendRequest(err.into())
    }
}

/// A set of errors that can occur during parsing json payloads
#[derive(Debug, Display, From)]
pub enum JsonPayloadError {
    /// Content type error
    #[display(fmt = "Content type error")]
    ContentType,
    /// Deserialize error
    #[display(fmt = "Json deserialize error: {}", _0)]
    Deserialize(JsonError),
    /// Payload error
    #[display(fmt = "Error that occur during reading payload: {}", _0)]
    Payload(PayloadError),
}

/// Return `InternalServerError` for `JsonPayloadError`
impl ResponseError for JsonPayloadError {}

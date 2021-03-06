//! Query extractor

use std::sync::Arc;
use std::{fmt, ops};

use crate::http::error::Error;
use futures_util::future::{err, ok, Ready};
use serde::de;
use serde_urlencoded;

use crate::web::dev::Payload;
use crate::web::error::QueryPayloadError;
use crate::web::extract::FromRequest;
use crate::web::request::HttpRequest;

/// Extract typed information from the request's query.
///
/// **Note**: A query string consists of unordered `key=value` pairs, therefore it cannot
/// be decoded into any type which depends upon data ordering e.g. tuples or tuple-structs.
/// Attempts to do so will *fail at runtime*.
///
/// [**QueryConfig**](struct.QueryConfig.html) allows to configure extraction process.
///
/// ## Example
///
/// ```rust
/// use kayrx::web::{self, types, App};
/// use serde_derive::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// pub enum ResponseType {
///    Token,
///    Code
/// }
///
/// #[derive(Deserialize)]
/// pub struct AuthRequest {
///    id: u64,
///    response_type: ResponseType,
/// }
///
/// // Use `Query` extractor for query information (and destructure it within the signature).
/// // This handler gets called only if the request's query string contains a `username` field.
/// // The correct request for this handler would be `/index.html?id=64&response_type=Code"`.
/// 
/// async fn index(types::Query(info): types::Query<AuthRequest>) -> String {
///     format!("Authorization request for client with id={} and type={:?}!", info.id, info.response_type)
/// }
///
/// fn main() {
///     let app = App::new().service(
///        web::resource("/index.html").route(web::get().to(index))); // <- use `Query` extractor
/// }
/// ```
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Query<T>(pub T);

impl<T> Query<T> {
    /// Deconstruct to a inner value
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Get query parameters from the path
    pub fn from_query(query_str: &str) -> Result<Self, QueryPayloadError>
    where
        T: de::DeserializeOwned,
    {
        serde_urlencoded::from_str::<T>(query_str)
            .map(|val| Ok(Query(val)))
            .unwrap_or_else(move |e| Err(QueryPayloadError::Deserialize(e)))
    }
}

impl<T> ops::Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::DerefMut for Query<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Query<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Query<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Extract typed information from the request's query.
///
/// ## Example
///
/// ```rust
/// use kayrx::web::{self, types, App};
/// use serde_derive::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// pub enum ResponseType {
///    Token,
///    Code
/// }
///
/// #[derive(Deserialize)]
/// pub struct AuthRequest {
///    id: u64,
///    response_type: ResponseType,
/// }
///
/// // Use `Query` extractor for query information.
/// // This handler get called only if request's query contains `username` field
/// // The correct request for this handler would be `/index.html?id=64&response_type=Code"`
/// async fn index(info: types::Query<AuthRequest>) -> String {
///     format!("Authorization request for client with id={} and type={:?}!", info.id, info.response_type)
/// }
///
/// fn main() {
///     let app = App::new().service(
///        web::resource("/index.html")
///            .route(web::get().to(index))); // <- use `Query` extractor
/// }
/// ```
impl<T> FromRequest for Query<T>
where
    T: de::DeserializeOwned,
{
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = QueryConfig;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let error_handler = req
            .app_data::<Self::Config>()
            .map(|c| c.ehandler.clone())
            .unwrap_or(None);

        serde_urlencoded::from_str::<T>(req.query_string())
            .map(|val| ok(Query(val)))
            .unwrap_or_else(move |e| {
                let e = QueryPayloadError::Deserialize(e);

                log::debug!(
                    "Failed during Query extractor deserialization. \
                     Request path: {:?}",
                    req.path()
                );

                let e = if let Some(error_handler) = error_handler {
                    (error_handler)(e, req)
                } else {
                    e.into()
                };

                err(e)
            })
    }
}

/// Query extractor configuration
///
/// ## Example
///
/// ```rust
/// use kayrx::web::{error, self, types, App, FromRequest, HttpResponse};
/// use serde_derive::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Info {
///     username: String,
/// }
///
/// /// deserialize `Info` from request's querystring
/// async fn index(info: types::Query<Info>) -> String {
///     format!("Welcome {}!", info.username)
/// }
///
/// fn main() {
///     let app = App::new().service(
///         web::resource("/index.html").app_data(
///             // change query extractor configuration
///             types::Query::<Info>::configure(|cfg| {
///                 cfg.error_handler(|err, req| {  // <- create custom error response
///                     error::InternalError::from_response(
///                         err, HttpResponse::Conflict().finish()).into()
///                 })
///             }))
///             .route(web::post().to(index))
///     );
/// }
/// ```
#[derive(Clone)]
pub struct QueryConfig {
    ehandler:
        Option<Arc<dyn Fn(QueryPayloadError, &HttpRequest) -> Error + Send + Sync>>,
}

impl QueryConfig {
    /// Set custom error handler
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(QueryPayloadError, &HttpRequest) -> Error + Send + Sync + 'static,
    {
        self.ehandler = Some(Arc::new(f));
        self
    }
}

impl Default for QueryConfig {
    fn default() -> Self {
        QueryConfig { ehandler: None }
    }
}

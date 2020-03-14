 
//! Path extractor
use std::sync::Arc;
use std::{fmt, ops};

use crate::http::error::{Error, ErrorNotFound};
use crate::router::PathDeserializer;
use futures_util::future::{ready, Ready};
use serde::de;

use crate::web::dev::Payload;
use crate::web::error::PathError;
use crate::web::request::HttpRequest;
use crate::web::FromRequest;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
/// Extract typed information from the request's path.
///
/// [**PathConfig**](struct.PathConfig.html) allows to configure extraction process.
///
/// ## Example
///
/// ```rust
/// use kayrx::web::{self, types, App};
///
/// /// extract path info from "/{username}/{count}/index.html" url
/// /// {username} - deserializes to a String
/// /// {count} -  - deserializes to a u32
/// async fn index(info: types::Path<(String, u32)>) -> String {
///     format!("Welcome {}! {}", info.0, info.1)
/// }
///
/// fn main() {
///     let app = App::new().service(
///         web::resource("/{username}/{count}/index.html") // <- define path parameters
///              .route(web::get().to(index))               // <- register handler with `Path` extractor
///     );
/// }
/// ```
///
/// It is possible to extract path information to a specific type that
/// implements `Deserialize` trait from *serde*.
///
/// ```rust
/// use kayrx::web::{self, types, App, Error};
/// use serde_derive::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Info {
///     username: String,
/// }
///
/// /// extract `Info` from a path using serde
/// async fn index(info: types::Path<Info>) -> Result<String, Error> {
///     Ok(format!("Welcome {}!", info.username))
/// }
///
/// fn main() {
///     let app = App::new().service(
///         web::resource("/{username}/index.html") // <- define path parameters
///              .route(web::get().to(index)) // <- use handler with Path` extractor
///     );
/// }
/// ```
pub struct Path<T> {
    inner: T,
}

impl<T> Path<T> {
    /// Deconstruct to an inner value
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> AsRef<T> for Path<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> ops::Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> ops::DerefMut for Path<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> From<T> for Path<T> {
    fn from(inner: T) -> Path<T> {
        Path { inner }
    }
}

impl<T: fmt::Debug> fmt::Debug for Path<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Path<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Extract typed information from the request's path.
///
/// ## Example
///
/// ```rust
/// use kayrx::web::{self, types, App};
///
/// /// extract path info from "/{username}/{count}/index.html" url
/// /// {username} - deserializes to a String
/// /// {count} -  - deserializes to a u32
/// async fn index(info: types::Path<(String, u32)>) -> String {
///     format!("Welcome {}! {}", info.0, info.1)
/// }
///
/// fn main() {
///     let app = App::new().service(
///         web::resource("/{username}/{count}/index.html") // <- define path parameters
///              .route(web::get().to(index)) // <- register handler with `Path` extractor
///     );
/// }
/// ```
///
/// It is possible to extract path information to a specific type that
/// implements `Deserialize` trait from *serde*.
///
/// ```rust
/// use kayrx::web::{self, types, App, Error};
/// use serde_derive::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Info {
///     username: String,
/// }
///
/// /// extract `Info` from a path using serde
/// async fn index(info: types::Path<Info>) -> Result<String, Error> {
///     Ok(format!("Welcome {}!", info.username))
/// }
///
/// fn main() {
///     let app = App::new().service(
///         web::resource("/{username}/index.html") // <- define path parameters
///              .route(web::get().to(index)) // <- use handler with Path` extractor
///     );
/// }
/// ```
impl<T> FromRequest for Path<T>
where
    T: de::DeserializeOwned,
{
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = PathConfig;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let error_handler = req
            .app_data::<Self::Config>()
            .map(|c| c.ehandler.clone())
            .unwrap_or(None);

        ready(
            de::Deserialize::deserialize(PathDeserializer::new(req.match_info()))
                .map(|inner| Path { inner })
                .map_err(move |e| {
                    log::debug!(
                        "Failed during Path extractor deserialization. \
                         Request path: {:?}",
                        req.path()
                    );
                    if let Some(error_handler) = error_handler {
                        let e = PathError::Deserialize(e);
                        (error_handler)(e, req)
                    } else {
                        ErrorNotFound(e)
                    }
                }),
        )
    }
}

/// Path extractor configuration
///
/// ```rust
/// use kayrx::web::types::PathConfig;
/// use kayrx::web::{ error, self, types, App, FromRequest, HttpResponse};
/// use serde_derive::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// enum Folder {
///     #[serde(rename = "inbox")]
///     Inbox,
///     #[serde(rename = "outbox")]
///     Outbox,
/// }
///
/// // deserialize `Info` from request's path
/// async fn index(folder: types::Path<Folder>) -> String {
///     format!("Selected folder: {:?}!", folder)
/// }
///
/// fn main() {
///     let app = App::new().service(
///         web::resource("/messages/{folder}")
///             .app_data(PathConfig::default().error_handler(|err, req| {
///                 error::InternalError::from_response(
///                     err,
///                     HttpResponse::Conflict().finish(),
///                 )
///                 .into()
///             }))
///             .route(web::post().to(index)),
///     );
/// }
/// ```
#[derive(Clone)]
pub struct PathConfig {
    ehandler: Option<Arc<dyn Fn(PathError, &HttpRequest) -> Error + Send + Sync>>,
}

impl PathConfig {
    /// Set custom error handler
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(PathError, &HttpRequest) -> Error + Send + Sync + 'static,
    {
        self.ehandler = Some(Arc::new(f));
        self
    }
}

impl Default for PathConfig {
    fn default() -> Self {
        PathConfig { ehandler: None }
    }
}

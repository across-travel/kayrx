//! Multipart payload support
use crate::web::{dev::Payload, FromRequest, HttpRequest};
use futures_util::future::{ok, Ready};
use crate::http::error::Error;

use super::server::Multipart;

/// Get request's payload as multipart stream
///
/// Content-type: multipart/form-data;
///
/// ## Server example
///
/// ```rust
/// use futures::{Stream, StreamExt};
/// use kayrx::web::{self, HttpResponse, Error};
/// use kayrx::web::multipart as mp;
///
/// async fn index(mut payload: mp::Multipart) -> Result<HttpResponse, Error> {
///     // iterate over multipart stream
///     while let Some(item) = payload.next().await {
///            let mut field = item?;
///
///            // Field in turn is stream of *Bytes* object
///            while let Some(chunk) = field.next().await {
///                println!("-- CHUNK: \n{:?}", std::str::from_utf8(&chunk?));
///            }
///     }
///     Ok(HttpResponse::Ok().into())
/// }
/// # fn main() {}
/// ```
impl FromRequest for Multipart {
    type Error = Error;
    type Future = Ready<Result<Multipart, Error>>;
    type Config = ();

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        ok(Multipart::new(req.headers(), payload.take()))
    }
}
//! `Middleware` to normalize request's URI
use std::task::{Context, Poll};
use bytes::Bytes;
use futures_util::future::{ok, Ready};
use regex::Regex;

use crate::http::{PathAndQuery, Uri, error::Error};
use crate::service::{Service, Transform};
use crate::web::service::{ServiceRequest, ServiceResponse};

#[derive(Default, Clone, Copy)]
/// `Middleware` to normalize request's URI in place
///
/// Performs following:
///
/// - Merges multiple slashes into one.
///
/// ```rust
/// use kayrx::web::{self, middleware, App, HttpResponse};
/// use kayrx::http;
///
/// # fn main() {
/// let app = App::new()
///     .wrap(middleware::NormalizePath)
///     .service(
///         web::resource("/test")
///             .route(web::get().to(|| HttpResponse::Ok()))
///             .route(web::method(http::Method::HEAD).to(|| HttpResponse::MethodNotAllowed()))
///     );
/// # }
/// ```

pub struct NormalizePath;

impl<S, B> Transform<S> for NormalizePath
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = NormalizePathNormalization<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(NormalizePathNormalization {
            service,
            merge_slash: Regex::new("//+").unwrap(),
        })
    }
}

pub struct NormalizePathNormalization<S> {
    service: S,
    merge_slash: Regex,
}

impl<S, B> Service for NormalizePathNormalization<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let head = req.head_mut();
        let path = head.uri.path();
        let original_len = path.len();
        let path = self.merge_slash.replace_all(path, "/");

        if original_len != path.len() {
            let mut parts = head.uri.clone().into_parts();
            let pq = parts.path_and_query.as_ref().unwrap();

            let path = if let Some(q) = pq.query() {
                Bytes::from(format!("{}?{}", path, q))
            } else {
                Bytes::copy_from_slice(path.as_bytes())
            };
            parts.path_and_query = Some(PathAndQuery::from_maybe_shared(path).unwrap());

            let uri = Uri::from_parts(parts).unwrap();
            req.match_info_mut().get_mut().update(&uri);
            req.head_mut().uri = uri;
        }

        self.service.call(req)
    }
}

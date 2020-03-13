//! Custom handlers service for responses.
use std::rc::Rc;
use std::task::{Context, Poll};

use crate::service::{Service, Transform};
use futures_util::future::{ok, FutureExt, LocalBoxFuture, Ready};
use fxhash::FxHashMap;

use crate::web::dev::{ServiceRequest, ServiceResponse};
use crate::web::error::{Error, Result};
use crate::http::StatusCode;

/// Error handler response
pub enum ErrorHandlerResponse<B> {
    /// New http response got generated
    Response(ServiceResponse<B>),
    /// Result is a future that resolves to a new http response
    Future(LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>>),
}

type ErrorHandler<B> = dyn Fn(ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>>;

/// `Middleware` for allowing custom handlers for responses.
///
/// You can use `ErrorHandlers::handler()` method  to register a custom error
/// handler for specific status code. You can modify existing response or
/// create completely new one.
///
/// ## Example
///
/// ```rust
/// use kayrx::web::middleware::errhandlers::{ErrorHandlers, ErrorHandlerResponse};
/// use kayrx::web::{web, dev, App, HttpRequest, HttpResponse, Result};
/// use kayrx::http;
///
/// fn render_500<B>(mut res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
///     res.response_mut()
///        .headers_mut()
///        .insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("Error"));
///     Ok(ErrorHandlerResponse::Response(res))
/// }
///
/// # fn main() {
/// let app = App::new()
///     .wrap(
///         ErrorHandlers::new()
///             .handler(http::StatusCode::INTERNAL_SERVER_ERROR, render_500),
///     )
///     .service(web::resource("/test")
///         .route(web::get().to(|| HttpResponse::Ok()))
///         .route(web::head().to(|| HttpResponse::MethodNotAllowed())
///     ));
/// # }
/// ```
pub struct ErrorHandlers<B> {
    handlers: Rc<FxHashMap<StatusCode, Box<ErrorHandler<B>>>>,
}

impl<B> Default for ErrorHandlers<B> {
    fn default() -> Self {
        ErrorHandlers {
            handlers: Rc::new(FxHashMap::default()),
        }
    }
}

impl<B> ErrorHandlers<B> {
    /// Construct new `ErrorHandlers` instance
    pub fn new() -> Self {
        ErrorHandlers::default()
    }

    /// Register error handler for specified status code
    pub fn handler<F>(mut self, status: StatusCode, handler: F) -> Self
    where
        F: Fn(ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> + 'static,
    {
        Rc::get_mut(&mut self.handlers)
            .unwrap()
            .insert(status, Box::new(handler));
        self
    }
}

impl<S, B> Transform<S> for ErrorHandlers<B>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ErrorHandlersMiddleware<S, B>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ErrorHandlersMiddleware {
            service,
            handlers: self.handlers.clone(),
        })
    }
}

#[doc(hidden)]
pub struct ErrorHandlersMiddleware<S, B> {
    service: S,
    handlers: Rc<FxHashMap<StatusCode, Box<ErrorHandler<B>>>>,
}

impl<S, B> Service for ErrorHandlersMiddleware<S, B>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let handlers = self.handlers.clone();
        let fut = self.service.call(req);

        async move {
            let res = fut.await?;

            if let Some(handler) = handlers.get(&res.status()) {
                match handler(res) {
                    Ok(ErrorHandlerResponse::Response(res)) => Ok(res),
                    Ok(ErrorHandlerResponse::Future(fut)) => fut.await,
                    Err(e) => Err(e),
                }
            } else {
                Ok(res)
            }
        }
        .boxed_local()
    }
}

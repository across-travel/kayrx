//! `Middleware` for conditionally enables another middleware.
use std::task::{Context, Poll};

use crate::service::{Service, Transform};
use futures_util::future::{ok, Either, FutureExt, LocalBoxFuture};

/// `Middleware` for conditionally enables another middleware.
/// The controled middleware must not change the `Service` interfaces.
/// This means you cannot control such middlewares like `Logger` or `Compress`.
///
/// ## Usage
///
/// ```rust
/// use kayrx::web::middleware::{Condition, NormalizePath};
/// use kayrx::web::App;
///
/// # fn main() {
/// let enable_normalize = std::env::var("NORMALIZE_PATH") == Ok("true".into());
/// let app = App::new()
///     .wrap(Condition::new(enable_normalize, NormalizePath));
/// # }
/// ```
pub struct Condition<T> {
    trans: T,
    enable: bool,
}

impl<T> Condition<T> {
    pub fn new(enable: bool, trans: T) -> Self {
        Self { trans, enable }
    }
}

impl<S, T> Transform<S> for Condition<T>
where
    S: Service + 'static,
    T: Transform<S, Request = S::Request, Response = S::Response, Error = S::Error>,
    T::Future: 'static,
    T::InitError: 'static,
    T::Transform: 'static,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type InitError = T::InitError;
    type Transform = ConditionMiddleware<T::Transform, S>;
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        if self.enable {
            let f = self.trans.new_transform(service).map(|res| {
                res.map(
                    ConditionMiddleware::Enable as fn(T::Transform) -> Self::Transform,
                )
            });
            Either::Left(f)
        } else {
            Either::Right(ok(ConditionMiddleware::Disable(service)))
        }
        .boxed_local()
    }
}

pub enum ConditionMiddleware<E, D> {
    Enable(E),
    Disable(D),
}

impl<E, D> Service for ConditionMiddleware<E, D>
where
    E: Service,
    D: Service<Request = E::Request, Response = E::Response, Error = E::Error>,
{
    type Request = E::Request;
    type Response = E::Response;
    type Error = E::Error;
    type Future = Either<E::Future, D::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        use ConditionMiddleware::*;
        match self {
            Enable(service) => service.poll_ready(cx),
            Disable(service) => service.poll_ready(cx),
        }
    }

    fn call(&mut self, req: E::Request) -> Self::Future {
        use ConditionMiddleware::*;
        match self {
            Enable(service) => Either::Left(service.call(req)),
            Disable(service) => Either::Right(service.call(req)),
        }
    }
}

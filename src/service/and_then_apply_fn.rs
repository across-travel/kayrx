use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::service::cell::Cell;
use crate::service::{Service, ServiceFactory};

/// `Apply` service combinator
pub struct AndThenApplyFn<A, B, F, Fut, Res, Err>
where
    A: Service,
    B: Service,
    F: FnMut(A::Response, &mut B) -> Fut,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error> + From<B::Error>,
{
    a: A,
    b: Cell<(B, F)>,
    r: PhantomData<(Fut, Res, Err)>,
}

impl<A, B, F, Fut, Res, Err> AndThenApplyFn<A, B, F, Fut, Res, Err>
where
    A: Service,
    B: Service,
    F: FnMut(A::Response, &mut B) -> Fut,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error> + From<B::Error>,
{
    /// Create new `Apply` combinator
    pub(crate) fn new(a: A, b: B, f: F) -> Self {
        Self {
            a,
            b: Cell::new((b, f)),
            r: PhantomData,
        }
    }
}

impl<A, B, F, Fut, Res, Err> Clone for AndThenApplyFn<A, B, F, Fut, Res, Err>
where
    A: Service + Clone,
    B: Service,
    F: FnMut(A::Response, &mut B) -> Fut,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error> + From<B::Error>,
{
    fn clone(&self) -> Self {
        AndThenApplyFn {
            a: self.a.clone(),
            b: self.b.clone(),
            r: PhantomData,
        }
    }
}

impl<A, B, F, Fut, Res, Err> Service for AndThenApplyFn<A, B, F, Fut, Res, Err>
where
    A: Service,
    B: Service,
    F: FnMut(A::Response, &mut B) -> Fut,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error> + From<B::Error>,
{
    type Request = A::Request;
    type Response = Res;
    type Error = Err;
    type Future = AndThenApplyFnFuture<A, B, F, Fut, Res, Err>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let not_ready = self.a.poll_ready(cx)?.is_pending();
        if self.b.get_mut().0.poll_ready(cx)?.is_pending() || not_ready {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn call(&mut self, req: A::Request) -> Self::Future {
        AndThenApplyFnFuture {
            state: State::A(self.a.call(req), Some(self.b.clone())),
        }
    }
}

#[pin_project::pin_project]
pub struct AndThenApplyFnFuture<A, B, F, Fut, Res, Err>
where
    A: Service,
    B: Service,
    F: FnMut(A::Response, &mut B) -> Fut,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error>,
    Err: From<B::Error>,
{
    #[pin]
    state: State<A, B, F, Fut, Res, Err>,
}

#[pin_project::pin_project]
enum State<A, B, F, Fut, Res, Err>
where
    A: Service,
    B: Service,
    F: FnMut(A::Response, &mut B) -> Fut,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error>,
    Err: From<B::Error>,
{
    A(#[pin] A::Future, Option<Cell<(B, F)>>),
    B(#[pin] Fut),
    Empty,
}

impl<A, B, F, Fut, Res, Err> Future for AndThenApplyFnFuture<A, B, F, Fut, Res, Err>
where
    A: Service,
    B: Service,
    F: FnMut(A::Response, &mut B) -> Fut,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error> + From<B::Error>,
{
    type Output = Result<Res, Err>;

    #[pin_project::project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.as_mut().project();

        #[project]
        match this.state.as_mut().project() {
            State::A(fut, b) => match fut.poll(cx)? {
                Poll::Ready(res) => {
                    let mut b = b.take().unwrap();
                    this.state.set(State::Empty);
                    let b = b.get_mut();
                    let fut = (&mut b.1)(res, &mut b.0);
                    this.state.set(State::B(fut));
                    self.poll(cx)
                }
                Poll::Pending => Poll::Pending,
            },
            State::B(fut) => fut.poll(cx).map(|r| {
                this.state.set(State::Empty);
                r
            }),
            State::Empty => panic!("future must not be polled after it returned `Poll::Ready`"),
        }
    }
}

/// `AndThenApplyFn` service factory
pub struct AndThenApplyFnFactory<A, B, F, Fut, Res, Err> {
    a: A,
    b: B,
    f: F,
    r: PhantomData<(Fut, Res, Err)>,
}

impl<A, B, F, Fut, Res, Err> AndThenApplyFnFactory<A, B, F, Fut, Res, Err>
where
    A: ServiceFactory,
    B: ServiceFactory<Config = A::Config, InitError = A::InitError>,
    F: FnMut(A::Response, &mut B::Service) -> Fut + Clone,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error> + From<B::Error>,
{
    /// Create new `ApplyNewService` new service instance
    pub(crate) fn new(a: A, b: B, f: F) -> Self {
        Self {
            a: a,
            b: b,
            f: f,
            r: PhantomData,
        }
    }
}

impl<A, B, F, Fut, Res, Err> Clone for AndThenApplyFnFactory<A, B, F, Fut, Res, Err>
where
    A: Clone,
    B: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            a: self.a.clone(),
            b: self.b.clone(),
            f: self.f.clone(),
            r: PhantomData,
        }
    }
}

impl<A, B, F, Fut, Res, Err> ServiceFactory for AndThenApplyFnFactory<A, B, F, Fut, Res, Err>
where
    A: ServiceFactory,
    A::Config: Clone,
    B: ServiceFactory<Config = A::Config, InitError = A::InitError>,
    F: FnMut(A::Response, &mut B::Service) -> Fut + Clone,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error> + From<B::Error>,
{
    type Request = A::Request;
    type Response = Res;
    type Error = Err;
    type Service = AndThenApplyFn<A::Service, B::Service, F, Fut, Res, Err>;
    type Config = A::Config;
    type InitError = A::InitError;
    type Future = AndThenApplyFnFactoryResponse<A, B, F, Fut, Res, Err>;

    fn new_service(&self, cfg: A::Config) -> Self::Future {
        AndThenApplyFnFactoryResponse {
            a: None,
            b: None,
            f: self.f.clone(),
            fut_a: self.a.new_service(cfg.clone()),
            fut_b: self.b.new_service(cfg),
        }
    }
}

#[pin_project::pin_project]
pub struct AndThenApplyFnFactoryResponse<A, B, F, Fut, Res, Err>
where
    A: ServiceFactory,
    B: ServiceFactory<Config = A::Config, InitError = A::InitError>,
    F: FnMut(A::Response, &mut B::Service) -> Fut + Clone,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error>,
    Err: From<B::Error>,
{
    #[pin]
    fut_b: B::Future,
    #[pin]
    fut_a: A::Future,
    f: F,
    a: Option<A::Service>,
    b: Option<B::Service>,
}

impl<A, B, F, Fut, Res, Err> Future for AndThenApplyFnFactoryResponse<A, B, F, Fut, Res, Err>
where
    A: ServiceFactory,
    B: ServiceFactory<Config = A::Config, InitError = A::InitError>,
    F: FnMut(A::Response, &mut B::Service) -> Fut + Clone,
    Fut: Future<Output = Result<Res, Err>>,
    Err: From<A::Error> + From<B::Error>,
{
    type Output =
        Result<AndThenApplyFn<A::Service, B::Service, F, Fut, Res, Err>, A::InitError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if this.a.is_none() {
            if let Poll::Ready(service) = this.fut_a.poll(cx)? {
                *this.a = Some(service);
            }
        }

        if this.b.is_none() {
            if let Poll::Ready(service) = this.fut_b.poll(cx)? {
                *this.b = Some(service);
            }
        }

        if this.a.is_some() && this.b.is_some() {
            Poll::Ready(Ok(AndThenApplyFn {
                a: this.a.take().unwrap(),
                b: Cell::new((this.b.take().unwrap(), this.f.clone())),
                r: PhantomData,
            }))
        } else {
            Poll::Pending
        }
    }
}

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::krse::io::{AsyncRead, AsyncWrite};
use crate::codec::Framed2 as Framed;
use crate::service::{IntoService, Service};
use crate::framed;

use super::{Codec, Frame, Message};

pub struct Dispatcher<S, T>
where
    S: Service<Request = Frame, Response = Message> + 'static,
    T: AsyncRead + AsyncWrite,
{
    inner: framed::Dispatcher<S, T, Codec>,
}

impl<S, T> Dispatcher<S, T>
where
    T: AsyncRead + AsyncWrite,
    S: Service<Request = Frame, Response = Message>,
    S::Future: 'static,
    S::Error: 'static,
{
    pub fn new<F: IntoService<S>>(io: T, service: F) -> Self {
        Dispatcher {
            inner: framed::Dispatcher::new(Framed::new(io, Codec::new()), service),
        }
    }

    pub fn with<F: IntoService<S>>(framed: Framed<T, Codec>, service: F) -> Self {
        Dispatcher {
            inner: framed::Dispatcher::new(framed, service),
        }
    }
}

impl<S, T> Future for Dispatcher<S, T>
where
    T: AsyncRead + AsyncWrite,
    S: Service<Request = Frame, Response = Message>,
    S::Future: 'static,
    S::Error: 'static,
{
    type Output = Result<(), framed::DispatcherError<S::Error, Codec>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx)
    }
}

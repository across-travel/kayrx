use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::future::lazy;

use crate::server::server::Server;
use crate::fiber;

/// Different types of process signals
#[allow(dead_code)]
#[derive(PartialEq, Clone, Copy, Debug)]
pub(crate) enum Signal {
    /// SIGHUP
    Hup,
    /// SIGINT
    Int,
    /// SIGTERM
    Term,
    /// SIGQUIT
    Quit,
}

pub(crate) struct Signals {
    srv: Server,
    streams: Vec<(Signal, crate::krse::signal::unix::Signal)>,
}

impl Signals {
    pub(crate) fn start(srv: Server) -> io::Result<()> {
        fiber::spawn(lazy(|_| {
                use crate::krse::signal::unix;

                let mut streams = Vec::new();

                let sig_map = [
                    (unix::SignalKind::interrupt(), Signal::Int),
                    (unix::SignalKind::hangup(), Signal::Hup),
                    (unix::SignalKind::terminate(), Signal::Term),
                    (unix::SignalKind::quit(), Signal::Quit),
                ];

                for (kind, sig) in sig_map.iter() {
                    match unix::signal(*kind) {
                        Ok(stream) => streams.push((*sig, stream)),
                        Err(e) => log::error!(
                            "Can not initialize stream handler for {:?} err: {}",
                            sig,
                            e
                        ),
                    }
                }

                fiber::spawn(Signals { srv, streams })
        }));

        Ok(())
    }
}

impl Future for Signals {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
       
            for idx in 0..self.streams.len() {
                loop {
                    match self.streams[idx].1.poll_recv(cx) {
                        Poll::Ready(None) => return Poll::Ready(()),
                        Poll::Pending => break,
                        Poll::Ready(Some(_)) => {
                            let sig = self.streams[idx].0;
                            self.srv.signal(sig);
                        }
                    }
                }
            }
            Poll::Pending
    }
}
use std::task::{Context, Poll};
use std::time::Duration;
use kayrx::timer;
use kayrx::util::inflight::*;
use kayrx::service::{apply, fn_factory, Service, ServiceFactory};
use futures::future::{lazy, ok, FutureExt, LocalBoxFuture};

struct SleepService(Duration);

impl Service for SleepService {
    type Request = ();
    type Response = ();
    type Error = ();
    type Future = LocalBoxFuture<'static, Result<(), ()>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: ()) -> Self::Future {
        timer::delay_for(self.0)
            .then(|_| ok::<_, ()>(()))
            .boxed_local()
    }
}

#[kayrx::test]
async fn test_transform() {
    let wait_time = Duration::from_millis(50);

    let mut srv = InFlightService::new(1, SleepService(wait_time));
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));

    let res = srv.call(());
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Pending);

    let _ = res.await;
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));
}

#[kayrx::test]
async fn test_newtransform() {
    let wait_time = Duration::from_millis(50);

    let srv = apply(InFlight::new(1), fn_factory(|| ok(SleepService(wait_time))));

    let mut srv = srv.new_service(&()).await.unwrap();
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));

    let res = srv.call(());
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Pending);

    let _ = res.await;
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));
}
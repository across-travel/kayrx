use std::task::{Context, Poll};
use std::time::Duration;
use kayrx::timer;
use kayrx::util::timeout::*;
use kayrx::service::{apply, fn_factory, Service, ServiceFactory};
use futures::future::{ok, FutureExt, LocalBoxFuture};

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
async fn test_success() {
    let resolution = Duration::from_millis(100);
    let wait_time = Duration::from_millis(50);

    let mut timeout = TimeoutService::new(resolution, SleepService(wait_time));
    assert_eq!(timeout.call(()).await, Ok(()));
}

#[kayrx::test]
async fn test_timeout() {
    let resolution = Duration::from_millis(100);
    let wait_time = Duration::from_millis(500);

    let mut timeout = TimeoutService::new(resolution, SleepService(wait_time));
    assert_eq!(timeout.call(()).await, Err(TimeoutError::Timeout));
}

#[kayrx::test]
async fn test_timeout_newservice() {
    let resolution = Duration::from_millis(100);
    let wait_time = Duration::from_millis(500);

    let timeout = apply(
        Timeout::new(resolution),
        fn_factory(|| ok::<_, ()>(SleepService(wait_time))),
    );
    let mut srv = timeout.new_service(&()).await.unwrap();

    assert_eq!(srv.call(()).await, Err(TimeoutError::Timeout));
}
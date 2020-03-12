use futures_util::future::{lazy, ok, Ready};
use std::task::{Context, Poll};
use kayrx::service::{IntoServiceFactory, Service, ServiceFactory};

struct Srv;

impl Service for Srv {
    type Request = ();
    type Response = ();
    type Error = ();
    type Future = Ready<Result<(), ()>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: ()) -> Self::Future {
        ok(())
    }
}

#[kayrx::test]
async fn test_poll_ready() {
    let mut srv = Srv.map(|_| "ok");
    let res = lazy(|cx| srv.poll_ready(cx)).await;
    assert_eq!(res, Poll::Ready(Ok(())));
}

#[kayrx::test]
async fn test_call() {
    let mut srv = Srv.map(|_| "ok");
    let res = srv.call(()).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "ok");
}

#[kayrx::test]
async fn test_new_service() {
    let new_srv = (|| ok::<_, ()>(Srv)).into_factory().map(|_| "ok");
    let mut srv = new_srv.new_service(&()).await.unwrap();
    let res = srv.call(()).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), ("ok"));
}
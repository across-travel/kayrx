use futures_util::future::{err, lazy, ok, Ready};
use std::task::{Context, Poll};
use kayrx::service::{IntoServiceFactory, Service, ServiceFactory};

struct Srv;

impl Service for Srv {
    type Request = ();
    type Response = ();
    type Error = ();
    type Future = Ready<Result<(), ()>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err(()))
    }

    fn call(&mut self, _: ()) -> Self::Future {
        err(())
    }
}

#[kayrx::test]
async fn test_poll_ready() {
    let mut srv = Srv.map_err(|_| "error");
    let res = lazy(|cx| srv.poll_ready(cx)).await;
    assert_eq!(res, Poll::Ready(Err("error")));
}

#[kayrx::test]
async fn test_call() {
    let mut srv = Srv.map_err(|_| "error");
    let res = srv.call(()).await;
    assert!(res.is_err());
    assert_eq!(res.err().unwrap(), "error");
}

#[kayrx::test]
async fn test_new_service() {
    let new_srv = (|| ok::<_, ()>(Srv)).into_factory().map_err(|_| "error");
    let mut srv = new_srv.new_service(&()).await.unwrap();
    let res = srv.call(()).await;
    assert!(res.is_err());
    assert_eq!(res.err().unwrap(), "error");
}
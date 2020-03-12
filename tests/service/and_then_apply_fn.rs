use std::task::{Context, Poll};
use futures::future::{lazy, ok, Ready, TryFutureExt};
use kayrx::service::{fn_service, pipeline, pipeline_factory, Service, ServiceFactory};

#[derive(Clone)]
struct Srv;
impl Service for Srv {
    type Request = ();
    type Response = ();
    type Error = ();
    type Future = Ready<Result<(), ()>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        ok(req)
    }
}

#[kayrx::test]
async fn test_service() {
    let mut srv = pipeline(|r: &'static str| ok(r))
        .and_then_apply_fn(Srv, |req: &'static str, s| {
            s.call(()).map_ok(move |res| (req, res))
        });
    let res = lazy(|cx| srv.poll_ready(cx)).await;
    assert_eq!(res, Poll::Ready(Ok(())));

    let res = srv.call("srv").await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), ("srv", ()));
}

#[kayrx::test]
async fn test_service_factory() {
    let new_srv = pipeline_factory(|| ok::<_, ()>(fn_service(|r: &'static str| ok(r))))
        .and_then_apply_fn(
            || ok(Srv),
            |req: &'static str, s| s.call(()).map_ok(move |res| (req, res)),
        );
    let mut srv = new_srv.new_service(()).await.unwrap();
    let res = lazy(|cx| srv.poll_ready(cx)).await;
    assert_eq!(res, Poll::Ready(Ok(())));

    let res = srv.call("srv").await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), ("srv", ()));
}
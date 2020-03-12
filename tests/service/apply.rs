use std::task::{Context, Poll};

use futures::future::{lazy, ok, Ready};
use kayrx::service::{apply_fn, apply_fn_factory};

use kayrx::service::{pipeline, pipeline_factory, Service, ServiceFactory};

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

    fn call(&mut self, _: ()) -> Self::Future {
        ok(())
    }
}

#[kayrx::test]
async fn test_call() {
    let mut srv = pipeline(apply_fn(Srv, |req: &'static str, srv| {
        let fut = srv.call(());
        async move {
            let res = fut.await.unwrap();
            Ok((req, res))
        }
    }));

    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));

    let res = srv.call("srv").await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (("srv", ())));
}

#[kayrx::test]
async fn test_new_service() {
    let new_srv = pipeline_factory(apply_fn_factory(
        || ok::<_, ()>(Srv),
        |req: &'static str, srv| {
            let fut = srv.call(());
            async move {
                let res = fut.await.unwrap();
                Ok((req, res))
            }
        },
    ));

    let mut srv = new_srv.new_service(()).await.unwrap();

    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));

    let res = srv.call("srv").await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (("srv", ())));
}
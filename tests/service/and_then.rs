use std::cell::Cell;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures::future::{lazy, ok, ready, Ready};

use kayrx::service::{fn_factory, pipeline, pipeline_factory, Service, ServiceFactory};

struct Srv1(Rc<Cell<usize>>);

impl Service for Srv1 {
    type Request = &'static str;
    type Response = &'static str;
    type Error = ();
    type Future = Ready<Result<Self::Response, ()>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.set(self.0.get() + 1);
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: &'static str) -> Self::Future {
        ok(req)
    }
}

#[derive(Clone)]
struct Srv2(Rc<Cell<usize>>);

impl Service for Srv2 {
    type Request = &'static str;
    type Response = (&'static str, &'static str);
    type Error = ();
    type Future = Ready<Result<Self::Response, ()>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.set(self.0.get() + 1);
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: &'static str) -> Self::Future {
        ok((req, "srv2"))
    }
}

#[kayrx::test]
async fn test_poll_ready() {
    let cnt = Rc::new(Cell::new(0));
    let mut srv = pipeline(Srv1(cnt.clone())).and_then(Srv2(cnt.clone()));
    let res = lazy(|cx| srv.poll_ready(cx)).await;
    assert_eq!(res, Poll::Ready(Ok(())));
    assert_eq!(cnt.get(), 2);
}

#[kayrx::test]
async fn test_call() {
    let cnt = Rc::new(Cell::new(0));
    let mut srv = pipeline(Srv1(cnt.clone())).and_then(Srv2(cnt));
    let res = srv.call("srv1").await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (("srv1", "srv2")));
}

#[kayrx::test]
async fn test_new_service() {
    let cnt = Rc::new(Cell::new(0));
    let cnt2 = cnt.clone();
    let new_srv =
        pipeline_factory(fn_factory(move || ready(Ok::<_, ()>(Srv1(cnt2.clone())))))
            .and_then(move || ready(Ok(Srv2(cnt.clone()))));

    let mut srv = new_srv.new_service(()).await.unwrap();
    let res = srv.call("srv1").await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), ("srv1", "srv2"));
}
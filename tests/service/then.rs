use std::cell::Cell;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures_util::future::{err, lazy, ok, ready, Ready};

use kayrx::service::{pipeline, pipeline_factory, Service, ServiceFactory};

#[derive(Clone)]
struct Srv1(Rc<Cell<usize>>);

impl Service for Srv1 {
    type Request = Result<&'static str, &'static str>;
    type Response = &'static str;
    type Error = ();
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.set(self.0.get() + 1);
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Result<&'static str, &'static str>) -> Self::Future {
        match req {
            Ok(msg) => ok(msg),
            Err(_) => err(()),
        }
    }
}

struct Srv2(Rc<Cell<usize>>);

impl Service for Srv2 {
    type Request = Result<&'static str, ()>;
    type Response = (&'static str, &'static str);
    type Error = ();
    type Future = Ready<Result<Self::Response, ()>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.set(self.0.get() + 1);
        Poll::Ready(Err(()))
    }

    fn call(&mut self, req: Result<&'static str, ()>) -> Self::Future {
        match req {
            Ok(msg) => ok((msg, "ok")),
            Err(()) => ok(("srv2", "err")),
        }
    }
}

#[kayrx::test]
async fn test_poll_ready() {
    let cnt = Rc::new(Cell::new(0));
    let mut srv = pipeline(Srv1(cnt.clone())).then(Srv2(cnt.clone()));
    let res = lazy(|cx| srv.poll_ready(cx)).await;
    assert_eq!(res, Poll::Ready(Err(())));
    assert_eq!(cnt.get(), 2);
}

#[kayrx::test]
async fn test_call() {
    let cnt = Rc::new(Cell::new(0));
    let mut srv = pipeline(Srv1(cnt.clone())).then(Srv2(cnt));

    let res = srv.call(Ok("srv1")).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (("srv1", "ok")));

    let res = srv.call(Err("srv")).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (("srv2", "err")));
}

#[kayrx::test]
async fn test_factory() {
    let cnt = Rc::new(Cell::new(0));
    let cnt2 = cnt.clone();
    let blank = move || ready(Ok::<_, ()>(Srv1(cnt2.clone())));
    let factory = pipeline_factory(blank).then(move || ready(Ok(Srv2(cnt.clone()))));
    let mut srv = factory.new_service(&()).await.unwrap();
    let res = srv.call(Ok("srv1")).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (("srv1", "ok")));

    let res = srv.call(Err("srv")).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (("srv2", "err")));
}
use std::task::Poll;

use futures::future::{lazy, ok};
use kayrx::service::{fn_service, fn_factory_with_config};

use kayrx::service::{Service, ServiceFactory};

#[kayrx::test]
async fn test_fn_service() {
    let new_srv = fn_service(|()| ok::<_, ()>("srv"));

    let mut srv = new_srv.new_service(()).await.unwrap();
    let res = srv.call(()).await;
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "srv");
}

#[kayrx::test]
async fn test_fn_service_service() {
    let mut srv = fn_service(|()| ok::<_, ()>("srv"));

    let res = srv.call(()).await;
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "srv");
}

#[kayrx::test]
async fn test_fn_service_with_config() {
    let new_srv = fn_factory_with_config(|cfg: usize| {
        ok::<_, ()>(fn_service(move |()| ok::<_, ()>(("srv", cfg))))
    });

    let mut srv = new_srv.new_service(1).await.unwrap();
    let res = srv.call(()).await;
    assert_eq!(lazy(|cx| srv.poll_ready(cx)).await, Poll::Ready(Ok(())));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), ("srv", 1));
}
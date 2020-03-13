use kayrx::krse::sync::local::oneshot::*;
use futures::future::lazy;
use std::task::Poll;
use std::pin::Pin;
use std::future::Future;

#[kayrx::test]
async fn test_oneshot() {
    let (tx, rx) = channel();
    tx.send("test").unwrap();
    assert_eq!(rx.await.unwrap(), "test");

    let (tx, rx) = channel();
    assert!(!tx.is_canceled());
    drop(rx);
    assert!(tx.is_canceled());
    assert!(tx.send("test").is_err());

    let (tx, rx) = channel::<&'static str>();
    drop(tx);
    assert!(rx.await.is_err());

    let (tx, mut rx) = channel::<&'static str>();
    assert_eq!(lazy(|cx| Pin::new(&mut rx).poll(cx)).await, Poll::Pending);
    tx.send("test").unwrap();
    assert_eq!(rx.await.unwrap(), "test");

    let (tx, mut rx) = channel::<&'static str>();
    assert_eq!(lazy(|cx| Pin::new(&mut rx).poll(cx)).await, Poll::Pending);
    drop(tx);
    assert!(rx.await.is_err());
}

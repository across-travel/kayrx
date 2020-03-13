use kayrx::krse::sync::local::condition::*;
use futures::future::lazy;
use std::task::Poll;
use std::pin::Pin;
use std::future::Future;

#[kayrx::test]
async fn test_condition() {
    let mut cond = Condition::new();
    let mut waiter = cond.wait();
    assert_eq!(
        lazy(|cx| Pin::new(&mut waiter).poll(cx)).await,
        Poll::Pending
    );
    cond.notify();
    assert_eq!(waiter.await, ());

    let mut waiter = cond.wait();
    assert_eq!(
        lazy(|cx| Pin::new(&mut waiter).poll(cx)).await,
        Poll::Pending
    );
    let mut waiter2 = waiter.clone();
    assert_eq!(
        lazy(|cx| Pin::new(&mut waiter2).poll(cx)).await,
        Poll::Pending
    );

    drop(cond);
    assert_eq!(waiter.await, ());
    assert_eq!(waiter2.await, ());
}
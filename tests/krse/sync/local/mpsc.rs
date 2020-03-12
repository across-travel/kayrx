use kayrx::krse::sync::local::mpsc::*;
use futures::future::lazy;
use futures::{Stream, StreamExt};

#[kayrx::test]
async fn test_mpsc() {
    let (tx, mut rx) = channel();
    tx.send("test").unwrap();
    assert_eq!(rx.next().await.unwrap(), "test");

    let tx2 = tx.clone();
    tx2.send("test2").unwrap();
    assert_eq!(rx.next().await.unwrap(), "test2");

    assert_eq!(
        lazy(|cx| Pin::new(&mut rx).poll_next(cx)).await,
        Poll::Pending
    );
    drop(tx2);
    assert_eq!(
        lazy(|cx| Pin::new(&mut rx).poll_next(cx)).await,
        Poll::Pending
    );
    drop(tx);
    assert_eq!(rx.next().await, None);

    let (tx, rx) = channel();
    tx.send("test").unwrap();
    drop(rx);
    assert!(tx.send("test").is_err());

    let (mut tx, _) = channel();
    let tx2 = tx.clone();
    tx.close();
    assert!(tx.send("test").is_err());
    assert!(tx2.send("test").is_err());
}

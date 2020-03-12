use std::task::{Context, Poll};
use std::time::Duration;
use kayrx::fiber;
use kayrx::util::order::*;
use kayrx::service::Service;
use futures::channel::oneshot;
use futures::future::{lazy, poll_fn, FutureExt, LocalBoxFuture};

struct Srv;

impl Service for Srv {
    type Request = oneshot::Receiver<usize>;
    type Response = usize;
    type Error = ();
    type Future = LocalBoxFuture<'static, Result<usize, ()>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: oneshot::Receiver<usize>) -> Self::Future {
        req.map(|res| res.map_err(|_| ())).boxed_local()
    }
}

#[kayrx::test]
async fn test_inorder() {
    let (tx1, rx1) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();
    let (tx3, rx3) = oneshot::channel();
    let (tx_stop, rx_stop) = oneshot::channel();

    let h = std::thread::spawn(move || {
        let rx1 = rx1;
        let rx2 = rx2;
        let rx3 = rx3;
        let tx_stop = tx_stop;
        let _ = fiber::System::new("test").block_on(async {
            let mut srv = InOrderService::new(Srv);

            let _ = lazy(|cx| srv.poll_ready(cx)).await;
            let res1 = srv.call(rx1);
            let res2 = srv.call(rx2);
            let res3 = srv.call(rx3);

            fiber::spawn(async move {
                let _ = poll_fn(|cx| {
                    let _ = srv.poll_ready(cx);
                    Poll::<()>::Pending
                })
                .await;
            });

            assert_eq!(res1.await.unwrap(), 1);
            assert_eq!(res2.await.unwrap(), 2);
            assert_eq!(res3.await.unwrap(), 3);

            let _ = tx_stop.send(());
            fiber::System::current().stop();
        });
    });

    let _ = tx3.send(3);
    std::thread::sleep(Duration::from_millis(50));
    let _ = tx2.send(2);
    let _ = tx1.send(1);

    let _ = rx_stop.await;
    let _ = h.join();
}
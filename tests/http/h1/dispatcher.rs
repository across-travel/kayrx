use futures::future::{lazy, ok};
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use kayrx::http::cloneable::CloneableService;
use kayrx::http::config::ServiceConfig;
use kayrx::http::response::Response;
use kayrx::service::IntoService;
use kayrx::http::error::Error;
use kayrx::http::h1::{Dispatcher, ExpectHandler, UpgradeHandler};
use kayrx::http::test::TestBuffer;

#[kayrx::test]
async fn test_req_parse_err() {
    lazy(|cx| {
        let buf = TestBuffer::new("GET /test HTTP/1\r\n\r\n");

        let mut h1 = Dispatcher::<_, _, _, _, UpgradeHandler<TestBuffer>>::new(
            buf,
            ServiceConfig::default(),
            CloneableService::new(
                (|_| ok::<_, Error>(Response::Ok().finish())).into_service(),
            ),
            CloneableService::new(ExpectHandler),
            None,
            None,
            None,
        );
        match Pin::new(&mut h1).poll(cx) {
            Poll::Pending => panic!(),
            Poll::Ready(res) => assert!(res.is_err()),
        }

        if let DispatcherState::Normal(ref inner) = h1.inner {
            assert!(inner.flags.contains(Flags::READ_DISCONNECT));
            assert_eq!(&inner.io.write_buf[..26], b"HTTP/1.1 400 Bad Request\r\n");
        }
    })
    .await;
}

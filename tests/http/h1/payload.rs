use kayrx::http::h1::Payload;
use futures::future::poll_fn;
use bytes::Bytes;

#[kayrx::test]
async fn test_unread_data() {
    let (_, mut payload) = Payload::create(false);

    payload.unread_data(Bytes::from("data"));
    assert!(!payload.is_empty());
    assert_eq!(payload.len(), 4);

    assert_eq!(
        Bytes::from("data"),
        poll_fn(|cx| payload.readany(cx)).await.unwrap().unwrap()
    );
}
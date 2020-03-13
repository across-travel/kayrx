use kayrx::http::body::*;
use futures::future::poll_fn;
use serde_json::json;
use bytes::{Bytes, BytesMut};

#[kayrx::test]
async fn test_static_str() {
    assert_eq!(Body::from("").size(), BodySize::Sized(0));
    assert_eq!(Body::from("test").size(), BodySize::Sized(4));
    assert_eq!(Body::from("test").get_ref(), b"test");

    assert_eq!("test".size(), BodySize::Sized(4));
    assert_eq!(
        poll_fn(|cx| "test".poll_next(cx)).await.unwrap().ok(),
        Some(Bytes::from("test"))
    );
}

#[kayrx::test]
async fn test_static_bytes() {
    assert_eq!(Body::from(b"test".as_ref()).size(), BodySize::Sized(4));
    assert_eq!(Body::from(b"test".as_ref()).get_ref(), b"test");
    assert_eq!(
        Body::from_slice(b"test".as_ref()).size(),
        BodySize::Sized(4)
    );
    assert_eq!(Body::from_slice(b"test".as_ref()).get_ref(), b"test");

    assert_eq!((&b"test"[..]).size(), BodySize::Sized(4));
    assert_eq!(
        poll_fn(|cx| (&b"test"[..]).poll_next(cx))
            .await
            .unwrap()
            .ok(),
        Some(Bytes::from("test"))
    );
}

#[kayrx::test]
async fn test_vec() {
    assert_eq!(Body::from(Vec::from("test")).size(), BodySize::Sized(4));
    assert_eq!(Body::from(Vec::from("test")).get_ref(), b"test");

    assert_eq!(Vec::from("test").size(), BodySize::Sized(4));
    assert_eq!(
        poll_fn(|cx| Vec::from("test").poll_next(cx))
            .await
            .unwrap()
            .ok(),
        Some(Bytes::from("test"))
    );
}

#[kayrx::test]
async fn test_bytes() {
    let mut b = Bytes::from("test");
    assert_eq!(Body::from(b.clone()).size(), BodySize::Sized(4));
    assert_eq!(Body::from(b.clone()).get_ref(), b"test");

    assert_eq!(b.size(), BodySize::Sized(4));
    assert_eq!(
        poll_fn(|cx| b.poll_next(cx)).await.unwrap().ok(),
        Some(Bytes::from("test"))
    );
}

#[kayrx::test]
async fn test_bytes_mut() {
    let mut b = BytesMut::from("test");
    assert_eq!(Body::from(b.clone()).size(), BodySize::Sized(4));
    assert_eq!(Body::from(b.clone()).get_ref(), b"test");

    assert_eq!(b.size(), BodySize::Sized(4));
    assert_eq!(
        poll_fn(|cx| b.poll_next(cx)).await.unwrap().ok(),
        Some(Bytes::from("test"))
    );
}

#[kayrx::test]
async fn test_string() {
    let mut b = "test".to_owned();
    assert_eq!(Body::from(b.clone()).size(), BodySize::Sized(4));
    assert_eq!(Body::from(b.clone()).get_ref(), b"test");
    assert_eq!(Body::from(&b).size(), BodySize::Sized(4));
    assert_eq!(Body::from(&b).get_ref(), b"test");

    assert_eq!(b.size(), BodySize::Sized(4));
    assert_eq!(
        poll_fn(|cx| b.poll_next(cx)).await.unwrap().ok(),
        Some(Bytes::from("test"))
    );
}

#[kayrx::test]
async fn test_unit() {
    assert_eq!(().size(), BodySize::Empty);
    assert!(poll_fn(|cx| ().poll_next(cx)).await.is_none());
}

#[kayrx::test]
async fn test_box() {
    let mut val = Box::new(());
    assert_eq!(val.size(), BodySize::Empty);
    assert!(poll_fn(|cx| val.poll_next(cx)).await.is_none());
}

#[kayrx::test]
async fn test_body_eq() {
    assert!(Body::None == Body::None);
    assert!(Body::None != Body::Empty);
    assert!(Body::Empty == Body::Empty);
    assert!(Body::Empty != Body::None);
    assert!(
        Body::Bytes(Bytes::from_static(b"1"))
            == Body::Bytes(Bytes::from_static(b"1"))
    );
    assert!(Body::Bytes(Bytes::from_static(b"1")) != Body::None);
}

#[kayrx::test]
async fn test_body_debug() {
    assert!(format!("{:?}", Body::None).contains("Body::None"));
    assert!(format!("{:?}", Body::Empty).contains("Body::Empty"));
    assert!(format!("{:?}", Body::Bytes(Bytes::from_static(b"1"))).contains("1"));
}

#[kayrx::test]
async fn test_serde_json() {
    assert_eq!(
        Body::from(serde_json::Value::String("test".into())).size(),
        BodySize::Sized(6)
    );
    assert_eq!(
        Body::from(json!({"test-key":"test-value"})).size(),
        BodySize::Sized(25)
    );
}
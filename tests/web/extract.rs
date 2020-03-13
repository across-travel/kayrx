use kayrx::http::{header, error::Error};
use bytes::Bytes;
use serde::Deserialize;

use kayrx::web::*;
use kayrx::web::web::*;
use kayrx::web::test::TestRequest;

#[derive(Deserialize, Debug, PartialEq)]
struct Info {
    hello: String,
}

#[kayrx::test]
async fn test_option() {
    let (req, mut pl) = TestRequest::with_header(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded",
    )
    .data(FormConfig::default().limit(4096))
    .to_http_parts();

    let r = Option::<Form<Info>>::from_request(&req, &mut pl)
        .await
        .unwrap();
    assert_eq!(r, None);

    let (req, mut pl) = TestRequest::with_header(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded",
    )
    .header(header::CONTENT_LENGTH, "9")
    .set_payload(Bytes::from_static(b"hello=world"))
    .to_http_parts();

    let r = Option::<Form<Info>>::from_request(&req, &mut pl)
        .await
        .unwrap();
    assert_eq!(
        r,
        Some(Form(Info {
            hello: "world".into()
        }))
    );

    let (req, mut pl) = TestRequest::with_header(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded",
    )
    .header(header::CONTENT_LENGTH, "9")
    .set_payload(Bytes::from_static(b"bye=world"))
    .to_http_parts();

    let r = Option::<Form<Info>>::from_request(&req, &mut pl)
        .await
        .unwrap();
    assert_eq!(r, None);
}

#[kayrx::test]
async fn test_result() {
    let (req, mut pl) = TestRequest::with_header(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded",
    )
    .header(header::CONTENT_LENGTH, "11")
    .set_payload(Bytes::from_static(b"hello=world"))
    .to_http_parts();

    let r = Result::<Form<Info>, Error>::from_request(&req, &mut pl)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        r,
        Form(Info {
            hello: "world".into()
        })
    );

    let (req, mut pl) = TestRequest::with_header(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded",
    )
    .header(header::CONTENT_LENGTH, "9")
    .set_payload(Bytes::from_static(b"bye=world"))
    .to_http_parts();

    let r = Result::<Form<Info>, Error>::from_request(&req, &mut pl)
        .await
        .unwrap();
    assert!(r.is_err());
}
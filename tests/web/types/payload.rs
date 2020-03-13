use bytes::Bytes;
use kayrx::web::*;
use kayrx::web::web::PayloadConfig;
use kayrx::web::error::*;
use kayrx::http::header;
use kayrx::web::test::TestRequest;

#[kayrx::test]
async fn test_payload_config() {
    let req = TestRequest::default().to_http_request();
    let cfg = PayloadConfig::default().mimetype(mime::APPLICATION_JSON);
    assert!(cfg.check_mimetype(&req).is_err());

    let req = TestRequest::with_header(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded",
    )
    .to_http_request();
    assert!(cfg.check_mimetype(&req).is_err());

    let req = TestRequest::with_header(header::CONTENT_TYPE, "application/json")
        .to_http_request();
    assert!(cfg.check_mimetype(&req).is_ok());
}

#[kayrx::test]
async fn test_bytes() {
    let (req, mut pl) = TestRequest::with_header(header::CONTENT_LENGTH, "11")
        .set_payload(Bytes::from_static(b"hello=world"))
        .to_http_parts();

    let s = Bytes::from_request(&req, &mut pl).await.unwrap();
    assert_eq!(s, Bytes::from_static(b"hello=world"));
}

#[kayrx::test]
async fn test_string() {
    let (req, mut pl) = TestRequest::with_header(header::CONTENT_LENGTH, "11")
        .set_payload(Bytes::from_static(b"hello=world"))
        .to_http_parts();

    let s = String::from_request(&req, &mut pl).await.unwrap();
    assert_eq!(s, "hello=world");
}

#[kayrx::test]
async fn test_message_body() {
    let (req, mut pl) = TestRequest::with_header(header::CONTENT_LENGTH, "xxxx")
        .to_srv_request()
        .into_parts();
    let res = HttpMessageBody::new(&req, &mut pl).await;
    match res.err().unwrap() {
        PayloadError::UnknownLength => (),
        _ => unreachable!("error"),
    }

    let (req, mut pl) = TestRequest::with_header(header::CONTENT_LENGTH, "1000000")
        .to_srv_request()
        .into_parts();
    let res = HttpMessageBody::new(&req, &mut pl).await;
    match res.err().unwrap() {
        PayloadError::Overflow => (),
        _ => unreachable!("error"),
    }

    let (req, mut pl) = TestRequest::default()
        .set_payload(Bytes::from_static(b"test"))
        .to_http_parts();
    let res = HttpMessageBody::new(&req, &mut pl).await;
    assert_eq!(res.ok().unwrap(), Bytes::from_static(b"test"));

    let (req, mut pl) = TestRequest::default()
        .set_payload(Bytes::from_static(b"11111111111111"))
        .to_http_parts();
    let res = HttpMessageBody::new(&req, &mut pl).limit(5).await;
    match res.err().unwrap() {
        PayloadError::Overflow => (),
        _ => unreachable!("error"),
    }
}
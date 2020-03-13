use bytes::Bytes;
use serde::{Deserialize, Serialize};
use kayrx::http::StatusCode;
use kayrx::http::header::{HeaderValue, CONTENT_TYPE, CONTENT_LENGTH};
use kayrx::web::test::TestRequest;
use kayrx::web::error::*;
use kayrx::web::*;
use kayrx::web::web::*;
use kayrx::web::dev::*;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
struct Info {
    hello: String,
    counter: i64,
}

#[kayrx::test]
async fn test_form() {
    let (req, mut pl) =
        TestRequest::with_header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(CONTENT_LENGTH, "11")
            .set_payload(Bytes::from_static(b"hello=world&counter=123"))
            .to_http_parts();

    let Form(s) = Form::<Info>::from_request(&req, &mut pl).await.unwrap();
    assert_eq!(
        s,
        Info {
            hello: "world".into(),
            counter: 123
        }
    );
}

fn eq(err: UrlencodedError, other: UrlencodedError) -> bool {
    match err {
        UrlencodedError::Overflow { .. } => match other {
            UrlencodedError::Overflow { .. } => true,
            _ => false,
        },
        UrlencodedError::UnknownLength => match other {
            UrlencodedError::UnknownLength => true,
            _ => false,
        },
        UrlencodedError::ContentType => match other {
            UrlencodedError::ContentType => true,
            _ => false,
        },
        _ => false,
    }
}

#[kayrx::test]
async fn test_urlencoded_error() {
    let (req, mut pl) =
        TestRequest::with_header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(CONTENT_LENGTH, "xxxx")
            .to_http_parts();
    let info = UrlEncoded::<Info>::new(&req, &mut pl).await;
    assert!(eq(info.err().unwrap(), UrlencodedError::UnknownLength));

    let (req, mut pl) =
        TestRequest::with_header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(CONTENT_LENGTH, "1000000")
            .to_http_parts();
    let info = UrlEncoded::<Info>::new(&req, &mut pl).await;
    assert!(eq(
        info.err().unwrap(),
        UrlencodedError::Overflow { size: 0, limit: 0 }
    ));

    let (req, mut pl) = TestRequest::with_header(CONTENT_TYPE, "text/plain")
        .header(CONTENT_LENGTH, "10")
        .to_http_parts();
    let info = UrlEncoded::<Info>::new(&req, &mut pl).await;
    assert!(eq(info.err().unwrap(), UrlencodedError::ContentType));
}

#[kayrx::test]
async fn test_urlencoded() {
    let (req, mut pl) =
        TestRequest::with_header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(CONTENT_LENGTH, "11")
            .set_payload(Bytes::from_static(b"hello=world&counter=123"))
            .to_http_parts();

    let info = UrlEncoded::<Info>::new(&req, &mut pl).await.unwrap();
    assert_eq!(
        info,
        Info {
            hello: "world".to_owned(),
            counter: 123
        }
    );

    let (req, mut pl) = TestRequest::with_header(
        CONTENT_TYPE,
        "application/x-www-form-urlencoded; charset=utf-8",
    )
    .header(CONTENT_LENGTH, "11")
    .set_payload(Bytes::from_static(b"hello=world&counter=123"))
    .to_http_parts();

    let info = UrlEncoded::<Info>::new(&req, &mut pl).await.unwrap();
    assert_eq!(
        info,
        Info {
            hello: "world".to_owned(),
            counter: 123
        }
    );
}

#[kayrx::test]
async fn test_responder() {
    let req = TestRequest::default().to_http_request();

    let form = Form(Info {
        hello: "world".to_string(),
        counter: 123,
    });
    let resp = form.respond_to(&req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(CONTENT_TYPE).unwrap(),
        HeaderValue::from_static("application/x-www-form-urlencoded")
    );

    use kayrx::web::responder::tests::BodyTest;
    assert_eq!(resp.body().bin_ref(), b"hello=world&counter=123");
}
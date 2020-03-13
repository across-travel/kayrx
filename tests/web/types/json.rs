
    use bytes::Bytes;
    use serde::{Deserialize, Serialize};

    use kayrx::web::*;
    use kayrx::web::web::*;
    use kayrx::web::dev::*;
    use kayrx::http::error::*;
    use kayrx::web::error::*;
    use kayrx::http::header;
    use kayrx::web::test::{load_stream, TestRequest};
    use kayrx::http::Response;
    use kayrx::http::StatusCode;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct MyObject {
        name: String,
    }

    fn json_eq(err: JsonPayloadError, other: JsonPayloadError) -> bool {
        match err {
            JsonPayloadError::Overflow => match other {
                JsonPayloadError::Overflow => true,
                _ => false,
            },
            JsonPayloadError::ContentType => match other {
                JsonPayloadError::ContentType => true,
                _ => false,
            },
            _ => false,
        }
    }

    #[kayrx::test]
    async fn test_responder() {
        let req = TestRequest::default().to_http_request();

        let j = Json(MyObject {
            name: "test".to_string(),
        });
        let resp = j.respond_to(&req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            header::HeaderValue::from_static("application/json")
        );

        use kayrx::web::responder::tests::BodyTest;
        assert_eq!(resp.body().bin_ref(), b"{\"name\":\"test\"}");
    }

    #[kayrx::test]
    async fn test_custom_error_responder() {
        let (req, mut pl) = TestRequest::default()
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )
            .header(
                header::CONTENT_LENGTH,
                header::HeaderValue::from_static("16"),
            )
            .set_payload(Bytes::from_static(b"{\"name\": \"test\"}"))
            .app_data(JsonConfig::default().limit(10).error_handler(|err, _| {
                let msg = MyObject {
                    name: "invalid request".to_string(),
                };
                let resp = Response::BadRequest()
                    .body(serde_json::to_string(&msg).unwrap());
                InternalError::from_response(err, resp).into()
            }))
            .to_http_parts();

        let s = Json::<MyObject>::from_request(&req, &mut pl).await;
        let mut resp = Response::from_error(s.err().unwrap().into());
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body = load_stream(resp.take_body()).await.unwrap();
        let msg: MyObject = serde_json::from_slice(&body).unwrap();
        assert_eq!(msg.name, "invalid request");
    }

    #[kayrx::test]
    async fn test_extract() {
        let (req, mut pl) = TestRequest::default()
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )
            .header(
                header::CONTENT_LENGTH,
                header::HeaderValue::from_static("16"),
            )
            .set_payload(Bytes::from_static(b"{\"name\": \"test\"}"))
            .to_http_parts();

        let s = Json::<MyObject>::from_request(&req, &mut pl).await.unwrap();
        assert_eq!(s.name, "test");
        assert_eq!(
            s.into_inner(),
            MyObject {
                name: "test".to_string()
            }
        );

        let (req, mut pl) = TestRequest::default()
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )
            .header(
                header::CONTENT_LENGTH,
                header::HeaderValue::from_static("16"),
            )
            .set_payload(Bytes::from_static(b"{\"name\": \"test\"}"))
            .app_data(JsonConfig::default().limit(10))
            .to_http_parts();

        let s = Json::<MyObject>::from_request(&req, &mut pl).await;
        assert!(format!("{}", s.err().unwrap())
            .contains("Json payload size is bigger than allowed"));

        let (req, mut pl) = TestRequest::default()
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )
            .header(
                header::CONTENT_LENGTH,
                header::HeaderValue::from_static("16"),
            )
            .set_payload(Bytes::from_static(b"{\"name\": \"test\"}"))
            .app_data(
                JsonConfig::default()
                    .limit(10)
                    .error_handler(|_, _| JsonPayloadError::ContentType.into()),
            )
            .to_http_parts();
        let s = Json::<MyObject>::from_request(&req, &mut pl).await;
        assert!(format!("{}", s.err().unwrap()).contains("Content type error"));
    }

    #[kayrx::test]
    async fn test_json_body() {
        let (req, mut pl) = TestRequest::default().to_http_parts();
        let json = JsonBody::<MyObject>::new(&req, &mut pl, None).await;
        assert!(json_eq(json.err().unwrap(), JsonPayloadError::ContentType));

        let (req, mut pl) = TestRequest::default()
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/text"),
            )
            .to_http_parts();
        let json = JsonBody::<MyObject>::new(&req, &mut pl, None).await;
        assert!(json_eq(json.err().unwrap(), JsonPayloadError::ContentType));

        let (req, mut pl) = TestRequest::default()
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )
            .header(
                header::CONTENT_LENGTH,
                header::HeaderValue::from_static("10000"),
            )
            .to_http_parts();

        let json = JsonBody::<MyObject>::new(&req, &mut pl, None)
            .limit(100)
            .await;
        assert!(json_eq(json.err().unwrap(), JsonPayloadError::Overflow));

        let (req, mut pl) = TestRequest::default()
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )
            .header(
                header::CONTENT_LENGTH,
                header::HeaderValue::from_static("16"),
            )
            .set_payload(Bytes::from_static(b"{\"name\": \"test\"}"))
            .to_http_parts();

        let json = JsonBody::<MyObject>::new(&req, &mut pl, None).await;
        assert_eq!(
            json.ok().unwrap(),
            MyObject {
                name: "test".to_owned()
            }
        );
    }

    #[kayrx::test]
    async fn test_with_json_and_bad_content_type() {
        let (req, mut pl) = TestRequest::with_header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/plain"),
        )
        .header(
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("16"),
        )
        .set_payload(Bytes::from_static(b"{\"name\": \"test\"}"))
        .app_data(JsonConfig::default().limit(4096))
        .to_http_parts();

        let s = Json::<MyObject>::from_request(&req, &mut pl).await;
        assert!(s.is_err())
    }

    #[kayrx::test]
    async fn test_with_json_and_good_custom_content_type() {
        let (req, mut pl) = TestRequest::with_header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/plain"),
        )
        .header(
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("16"),
        )
        .set_payload(Bytes::from_static(b"{\"name\": \"test\"}"))
        .app_data(JsonConfig::default().content_type(|mime: mime::Mime| {
            mime.type_() == mime::TEXT && mime.subtype() == mime::PLAIN
        }))
        .to_http_parts();

        let s = Json::<MyObject>::from_request(&req, &mut pl).await;
        assert!(s.is_ok())
    }

    #[kayrx::test]
    async fn test_with_json_and_bad_custom_content_type() {
        let (req, mut pl) = TestRequest::with_header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/html"),
        )
        .header(
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("16"),
        )
        .set_payload(Bytes::from_static(b"{\"name\": \"test\"}"))
        .app_data(JsonConfig::default().content_type(|mime: mime::Mime| {
            mime.type_() == mime::TEXT && mime.subtype() == mime::PLAIN
        }))
        .to_http_parts();

        let s = Json::<MyObject>::from_request(&req, &mut pl).await;
        assert!(s.is_err())
    }

    use kayrx::http::StatusCode;
    use derive_more::Display;
    use serde::Deserialize;

    use kayrx::web::*;
    use kayrx::web::web::*;
    use kayrx::http::error::InternalError;
    use kayrx::web::test::TestRequest;
    use kayrx::http::Response;

    #[derive(Deserialize, Debug, Display)]
    struct Id {
        id: String,
    }

    #[kayrx::test]
    async fn test_service_request_extract() {
        let req = TestRequest::with_uri("/name/user1/").to_srv_request();
        assert!(Query::<Id>::from_query(&req.query_string()).is_err());

        let req = TestRequest::with_uri("/name/user1/?id=test").to_srv_request();
        let mut s = Query::<Id>::from_query(&req.query_string()).unwrap();

        assert_eq!(s.id, "test");
        assert_eq!(format!("{}, {:?}", s, s), "test, Id { id: \"test\" }");

        s.id = "test1".to_string();
        let s = s.into_inner();
        assert_eq!(s.id, "test1");
    }

    #[kayrx::test]
    async fn test_request_extract() {
        let req = TestRequest::with_uri("/name/user1/").to_srv_request();
        let (req, mut pl) = req.into_parts();
        assert!(Query::<Id>::from_request(&req, &mut pl).await.is_err());

        let req = TestRequest::with_uri("/name/user1/?id=test").to_srv_request();
        let (req, mut pl) = req.into_parts();

        let mut s = Query::<Id>::from_request(&req, &mut pl).await.unwrap();
        assert_eq!(s.id, "test");
        assert_eq!(format!("{}, {:?}", s, s), "test, Id { id: \"test\" }");

        s.id = "test1".to_string();
        let s = s.into_inner();
        assert_eq!(s.id, "test1");
    }

    #[kayrx::test]
    async fn test_custom_error_responder() {
        let req = TestRequest::with_uri("/name/user1/")
            .app_data(QueryConfig::default().error_handler(|e, _| {
                let resp = Response::UnprocessableEntity().finish();
                InternalError::from_response(e, resp).into()
            }))
            .to_srv_request();

        let (req, mut pl) = req.into_parts();
        let query = Query::<Id>::from_request(&req, &mut pl).await;

        assert!(query.is_err());
        assert_eq!(
            query
                .unwrap_err()
                .as_response_error()
                .error_response()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
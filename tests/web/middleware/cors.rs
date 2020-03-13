
    use kayrx::service::{fn_service, Transform};
    use kayrx::web::test::{self, TestRequest};
    use kayrx::web::middleware::dev::*;
    use futures::future::ok;
    use std::rc::Rc;
    use kayrx::web::dev::ServiceRequest;
    use kayrx::http::{Method, header,  StatusCode};
    use kayrx::http::Response as HttpResponse;


    #[kayrx::test]
    #[should_panic(expected = "Credentials are allowed, but the Origin is set to")]
    async fn cors_validates_illegal_allow_credentials() {
        let _cors = Cors::new().supports_credentials().send_wildcard().finish();
    }

    #[kayrx::test]
    async fn validate_origin_allows_all_origins() {
        let mut cors = Cors::new()
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();
        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[kayrx::test]
    async fn default() {
        let mut cors = Cors::default()
            .new_transform(test::ok_service())
            .await
            .unwrap();
        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[kayrx::test]
    async fn test_preflight() {
        let mut cors = Cors::new()
            .send_wildcard()
            .max_age(3600)
            .allowed_methods(vec![Method::GET, Method::OPTIONS, Method::POST])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .method(Method::OPTIONS)
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "X-Not-Allowed")
            .to_srv_request();

        assert!(cors.inner.validate_allowed_method(req.head()).is_err());
        assert!(cors.inner.validate_allowed_headers(req.head()).is_err());
        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "put")
            .method(Method::OPTIONS)
            .to_srv_request();

        assert!(cors.inner.validate_allowed_method(req.head()).is_err());
        assert!(cors.inner.validate_allowed_headers(req.head()).is_ok());

        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                "AUTHORIZATION,ACCEPT",
            )
            .method(Method::OPTIONS)
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(
            &b"*"[..],
            resp.headers()
                .get(&header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .as_bytes()
        );
        assert_eq!(
            &b"3600"[..],
            resp.headers()
                .get(&header::ACCESS_CONTROL_MAX_AGE)
                .unwrap()
                .as_bytes()
        );
        let hdr = resp
            .headers()
            .get(&header::ACCESS_CONTROL_ALLOW_HEADERS)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(hdr.contains("authorization"));
        assert!(hdr.contains("accept"));
        assert!(hdr.contains("content-type"));

        let methods = resp
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_METHODS)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(methods.contains("POST"));
        assert!(methods.contains("GET"));
        assert!(methods.contains("OPTIONS"));

        Rc::get_mut(&mut cors.inner).unwrap().preflight = false;

        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                "AUTHORIZATION,ACCEPT",
            )
            .method(Method::OPTIONS)
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // #[kayrx::test]
    // #[should_panic(expected = "MissingOrigin")]
    // async fn test_validate_missing_origin() {
    //    let cors = Cors::build()
    //        .allowed_origin("https://www.example.com")
    //        .finish();
    //    let mut req = HttpRequest::default();
    //    cors.start(&req).unwrap();
    // }

    #[kayrx::test]
    #[should_panic(expected = "OriginNotAllowed")]
    async fn test_validate_not_allowed_origin() {
        let cors = Cors::new()
            .allowed_origin("https://www.example.com")
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::with_header("Origin", "https://www.unknown.com")
            .method(Method::GET)
            .to_srv_request();
        cors.inner.validate_origin(req.head()).unwrap();
        cors.inner.validate_allowed_method(req.head()).unwrap();
        cors.inner.validate_allowed_headers(req.head()).unwrap();
    }

    #[kayrx::test]
    async fn test_validate_origin() {
        let mut cors = Cors::new()
            .allowed_origin("https://www.example.com")
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .method(Method::GET)
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[kayrx::test]
    async fn test_no_origin_response() {
        let mut cors = Cors::new()
            .disable_preflight()
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::default().method(Method::GET).to_srv_request();
        let resp = test::call_service(&mut cors, req).await;
        assert!(resp
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none());

        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .method(Method::OPTIONS)
            .to_srv_request();
        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(
            &b"https://www.example.com"[..],
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .as_bytes()
        );
    }

    #[kayrx::test]
    async fn test_response() {
        let exposed_headers = vec![header::AUTHORIZATION, header::ACCEPT];
        let mut cors = Cors::new()
            .send_wildcard()
            .disable_preflight()
            .max_age(3600)
            .allowed_methods(vec![Method::GET, Method::OPTIONS, Method::POST])
            .allowed_headers(exposed_headers.clone())
            .expose_headers(exposed_headers.clone())
            .allowed_header(header::CONTENT_TYPE)
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .method(Method::OPTIONS)
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(
            &b"*"[..],
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .as_bytes()
        );
        assert_eq!(
            &b"Origin"[..],
            resp.headers().get(header::VARY).unwrap().as_bytes()
        );

        {
            let headers = resp
                .headers()
                .get(header::ACCESS_CONTROL_EXPOSE_HEADERS)
                .unwrap()
                .to_str()
                .unwrap()
                .split(',')
                .map(|s| s.trim())
                .collect::<Vec<&str>>();

            for h in exposed_headers {
                assert!(headers.contains(&h.as_str()));
            }
        }

        let exposed_headers = vec![header::AUTHORIZATION, header::ACCEPT];
        let mut cors = Cors::new()
            .send_wildcard()
            .disable_preflight()
            .max_age(3600)
            .allowed_methods(vec![Method::GET, Method::OPTIONS, Method::POST])
            .allowed_headers(exposed_headers.clone())
            .expose_headers(exposed_headers.clone())
            .allowed_header(header::CONTENT_TYPE)
            .finish()
            .new_transform(fn_service(|req: ServiceRequest| {
                ok(req.into_response(
                    HttpResponse::Ok().header(header::VARY, "Accept").finish(),
                ))
            }))
            .await
            .unwrap();
        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .method(Method::OPTIONS)
            .to_srv_request();
        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(
            &b"Accept, Origin"[..],
            resp.headers().get(header::VARY).unwrap().as_bytes()
        );

        let mut cors = Cors::new()
            .disable_vary_header()
            .allowed_origin("https://www.example.com")
            .allowed_origin("https://www.google.com")
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::with_header("Origin", "https://www.example.com")
            .method(Method::OPTIONS)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .to_srv_request();
        let resp = test::call_service(&mut cors, req).await;

        let origins_str = resp
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!("https://www.example.com", origins_str);
    }

    #[kayrx::test]
    async fn test_multiple_origins() {
        let mut cors = Cors::new()
            .allowed_origin("https://example.com")
            .allowed_origin("https://example.org")
            .allowed_methods(vec![Method::GET])
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::with_header("Origin", "https://example.com")
            .method(Method::GET)
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(
            &b"https://example.com"[..],
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .as_bytes()
        );

        let req = TestRequest::with_header("Origin", "https://example.org")
            .method(Method::GET)
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(
            &b"https://example.org"[..],
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .as_bytes()
        );
    }

    #[kayrx::test]
    async fn test_multiple_origins_preflight() {
        let mut cors = Cors::new()
            .allowed_origin("https://example.com")
            .allowed_origin("https://example.org")
            .allowed_methods(vec![Method::GET])
            .finish()
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::with_header("Origin", "https://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .method(Method::OPTIONS)
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(
            &b"https://example.com"[..],
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .as_bytes()
        );

        let req = TestRequest::with_header("Origin", "https://example.org")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .method(Method::OPTIONS)
            .to_srv_request();

        let resp = test::call_service(&mut cors, req).await;
        assert_eq!(
            &b"https://example.org"[..],
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .as_bytes()
        );
    }
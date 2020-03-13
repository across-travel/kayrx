use bytes::Bytes;
use futures::future::ok;
use kayrx::service::Service;
use kayrx::web::dev::{Body, ResponseBody};
use kayrx::http::{header, HeaderValue, Method, StatusCode};
use kayrx::web::middleware::DefaultHeaders;
use kayrx::web::test::{call_service, init_service, read_body, TestRequest};
use kayrx::web::{guard, web, App, HttpRequest};
use kayrx::http::Response as HttpResponse;
use kayrx::web::dev::*;

#[kayrx::test]
async fn test_scope() {
    let mut srv = init_service(
        App::new().service(
            web::scope("/app")
                .service(web::resource("/path1").to(|| HttpResponse::Ok())),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_scope_root() {
    let mut srv = init_service(
        App::new().service(
            web::scope("/app")
                .service(web::resource("").to(|| HttpResponse::Ok()))
                .service(web::resource("/").to(|| HttpResponse::Created())),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/app/").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[kayrx::test]
async fn test_scope_root2() {
    let mut srv = init_service(App::new().service(
        web::scope("/app/").service(web::resource("").to(|| HttpResponse::Ok())),
    ))
    .await;

    let req = TestRequest::with_uri("/app").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let req = TestRequest::with_uri("/app/").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_scope_root3() {
    let mut srv = init_service(App::new().service(
        web::scope("/app/").service(web::resource("/").to(|| HttpResponse::Ok())),
    ))
    .await;

    let req = TestRequest::with_uri("/app").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let req = TestRequest::with_uri("/app/").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[kayrx::test]
async fn test_scope_route() {
    let mut srv = init_service(
        App::new().service(
            web::scope("app")
                .route("/path1", web::get().to(|| HttpResponse::Ok()))
                .route("/path1", web::delete().to(|| HttpResponse::Ok())),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/app/path1")
        .method(Method::DELETE)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/app/path1")
        .method(Method::POST)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[kayrx::test]
async fn test_scope_route_without_leading_slash() {
    let mut srv = init_service(
        App::new().service(
            web::scope("app").service(
                web::resource("path1")
                    .route(web::get().to(|| HttpResponse::Ok()))
                    .route(web::delete().to(|| HttpResponse::Ok())),
            ),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/app/path1")
        .method(Method::DELETE)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/app/path1")
        .method(Method::POST)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[kayrx::test]
async fn test_scope_guard() {
    let mut srv = init_service(
        App::new().service(
            web::scope("/app")
                .guard(guard::Get())
                .service(web::resource("/path1").to(|| HttpResponse::Ok())),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/path1")
        .method(Method::POST)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let req = TestRequest::with_uri("/app/path1")
        .method(Method::GET)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_scope_variable_segment() {
    let mut srv =
        init_service(App::new().service(web::scope("/ab-{project}").service(
            web::resource("/path1").to(|r: HttpRequest| {
                async move {
                    HttpResponse::Ok()
                        .body(format!("project: {}", &r.match_info()["project"]))
                }
            }),
        )))
        .await;

    let req = TestRequest::with_uri("/ab-project1/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    match resp.response().body() {
        ResponseBody::Body(Body::Bytes(ref b)) => {
            let bytes: Bytes = b.clone().into();
            assert_eq!(bytes, Bytes::from_static(b"project: project1"));
        }
        _ => panic!(),
    }

    let req = TestRequest::with_uri("/aa-project1/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[kayrx::test]
async fn test_nested_scope() {
    let mut srv = init_service(
        App::new().service(
            web::scope("/app")
                .service(web::scope("/t1").service(
                    web::resource("/path1").to(|| HttpResponse::Created()),
                )),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/t1/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[kayrx::test]
async fn test_nested_scope_no_slash() {
    let mut srv = init_service(
        App::new().service(
            web::scope("/app")
                .service(web::scope("t1").service(
                    web::resource("/path1").to(|| HttpResponse::Created()),
                )),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/t1/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[kayrx::test]
async fn test_nested_scope_root() {
    let mut srv = init_service(
        App::new().service(
            web::scope("/app").service(
                web::scope("/t1")
                    .service(web::resource("").to(|| HttpResponse::Ok()))
                    .service(web::resource("/").to(|| HttpResponse::Created())),
            ),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/t1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/app/t1/").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[kayrx::test]
async fn test_nested_scope_filter() {
    let mut srv = init_service(
        App::new().service(
            web::scope("/app").service(
                web::scope("/t1")
                    .guard(guard::Get())
                    .service(web::resource("/path1").to(|| HttpResponse::Ok())),
            ),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/t1/path1")
        .method(Method::POST)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let req = TestRequest::with_uri("/app/t1/path1")
        .method(Method::GET)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_nested_scope_with_variable_segment() {
    let mut srv = init_service(App::new().service(web::scope("/app").service(
        web::scope("/{project_id}").service(web::resource("/path1").to(
            |r: HttpRequest| {
                async move {
                    HttpResponse::Created()
                        .body(format!("project: {}", &r.match_info()["project_id"]))
                }
            },
        )),
    )))
    .await;

    let req = TestRequest::with_uri("/app/project_1/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    match resp.response().body() {
        ResponseBody::Body(Body::Bytes(ref b)) => {
            let bytes: Bytes = b.clone().into();
            assert_eq!(bytes, Bytes::from_static(b"project: project_1"));
        }
        _ => panic!(),
    }
}

#[kayrx::test]
async fn test_nested2_scope_with_variable_segment() {
    let mut srv = init_service(App::new().service(web::scope("/app").service(
        web::scope("/{project}").service(web::scope("/{id}").service(
            web::resource("/path1").to(|r: HttpRequest| {
                async move {
                    HttpResponse::Created().body(format!(
                        "project: {} - {}",
                        &r.match_info()["project"],
                        &r.match_info()["id"],
                    ))
                }
            }),
        )),
    )))
    .await;

    let req = TestRequest::with_uri("/app/test/1/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    match resp.response().body() {
        ResponseBody::Body(Body::Bytes(ref b)) => {
            let bytes: Bytes = b.clone().into();
            assert_eq!(bytes, Bytes::from_static(b"project: test - 1"));
        }
        _ => panic!(),
    }

    let req = TestRequest::with_uri("/app/test/1/path2").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[kayrx::test]
async fn test_default_resource() {
    let mut srv = init_service(
        App::new().service(
            web::scope("/app")
                .service(web::resource("/path1").to(|| HttpResponse::Ok()))
                .default_service(|r: ServiceRequest| {
                    ok(r.into_response(HttpResponse::BadRequest()))
                }),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/path2").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = TestRequest::with_uri("/path2").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[kayrx::test]
async fn test_default_resource_propagation() {
    let mut srv = init_service(
        App::new()
            .service(web::scope("/app1").default_service(
                web::resource("").to(|| HttpResponse::BadRequest()),
            ))
            .service(web::scope("/app2"))
            .default_service(|r: ServiceRequest| {
                ok(r.into_response(HttpResponse::MethodNotAllowed()))
            }),
    )
    .await;

    let req = TestRequest::with_uri("/non-exist").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    let req = TestRequest::with_uri("/app1/non-exist").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = TestRequest::with_uri("/app2/non-exist").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[kayrx::test]
async fn test_middleware() {
    let mut srv =
        init_service(
            App::new().service(
                web::scope("app")
                    .wrap(DefaultHeaders::new().header(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("0001"),
                    ))
                    .service(
                        web::resource("/test")
                            .route(web::get().to(|| HttpResponse::Ok())),
                    ),
            ),
        )
        .await;

    let req = TestRequest::with_uri("/app/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        HeaderValue::from_static("0001")
    );
}

#[kayrx::test]
async fn test_middleware_fn() {
    let mut srv = init_service(
        App::new().service(
            web::scope("app")
                .wrap_fn(|req, srv| {
                    let fut = srv.call(req);
                    async move {
                        let mut res = fut.await?;
                        res.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("0001"),
                        );
                        Ok(res)
                    }
                })
                .route("/test", web::get().to(|| HttpResponse::Ok())),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/app/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        HeaderValue::from_static("0001")
    );
}

#[kayrx::test]
async fn test_override_data() {
    let mut srv = init_service(App::new().data(1usize).service(
        web::scope("app").data(10usize).route(
            "/t",
            web::get().to(|data: web::Data<usize>| {
                assert_eq!(**data, 10);
                let _ = data.clone();
                HttpResponse::Ok()
            }),
        ),
    ))
    .await;

    let req = TestRequest::with_uri("/app/t").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_override_app_data() {
    let mut srv = init_service(App::new().app_data(web::Data::new(1usize)).service(
        web::scope("app").app_data(web::Data::new(10usize)).route(
            "/t",
            web::get().to(|data: web::Data<usize>| {
                assert_eq!(**data, 10);
                let _ = data.clone();
                HttpResponse::Ok()
            }),
        ),
    ))
    .await;

    let req = TestRequest::with_uri("/app/t").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_scope_config() {
    let mut srv =
        init_service(App::new().service(web::scope("/app").configure(|s| {
            s.route("/path1", web::get().to(|| HttpResponse::Ok()));
        })))
        .await;

    let req = TestRequest::with_uri("/app/path1").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_scope_config_2() {
    let mut srv =
        init_service(App::new().service(web::scope("/app").configure(|s| {
            s.service(web::scope("/v1").configure(|s| {
                s.route("/", web::get().to(|| HttpResponse::Ok()));
            }));
        })))
        .await;

    let req = TestRequest::with_uri("/app/v1/").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_url_for_external() {
    let mut srv =
        init_service(App::new().service(web::scope("/app").configure(|s| {
            s.service(web::scope("/v1").configure(|s| {
                s.external_resource(
                    "youtube",
                    "https://youtube.com/watch/{video_id}",
                );
                s.route(
                    "/",
                    web::get().to(|req: HttpRequest| {
                        async move {
                            HttpResponse::Ok().body(format!(
                                "{}",
                                req.url_for("youtube", &["xxxxxx"])
                                    .unwrap()
                                    .as_str()
                            ))
                        }
                    }),
                );
            }));
        })))
        .await;

    let req = TestRequest::with_uri("/app/v1/").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body, &b"https://youtube.com/watch/xxxxxx"[..]);
}

#[kayrx::test]
async fn test_url_for_nested() {
    let mut srv = init_service(App::new().service(web::scope("/a").service(
        web::scope("/b").service(web::resource("/c/{stuff}").name("c").route(
            web::get().to(|req: HttpRequest| {
                async move {
                    HttpResponse::Ok()
                        .body(format!("{}", req.url_for("c", &["12345"]).unwrap()))
                }
            }),
        )),
    )))
    .await;

    let req = TestRequest::with_uri("/a/b/c/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body,
        Bytes::from_static(b"http://localhost:8080/a/b/c/12345")
    );
}
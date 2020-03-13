use kayrx::service::Service;
use bytes::Bytes;
use futures::future::ok;

use kayrx::web::*;
use kayrx::web::dev::*;
use kayrx::http::{header, HeaderValue, Method, StatusCode};
use kayrx::web::middleware::DefaultHeaders;
use kayrx::web::service::ServiceRequest;
use kayrx::web::test::{call_service, init_service, read_body, TestRequest};
use kayrx::web::{web, HttpRequest};
use kayrx::http::Response as HttpResponse;

#[kayrx::test]
async fn test_default_resource() {
    let mut srv = init_service(
        App::new().service(web::resource("/test").to(|| HttpResponse::Ok())),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/blah").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let mut srv = init_service(
        App::new()
            .service(web::resource("/test").to(|| HttpResponse::Ok()))
            .service(
                web::resource("/test2")
                    .default_service(|r: ServiceRequest| {
                        ok(r.into_response(HttpResponse::Created()))
                    })
                    .route(web::get().to(|| HttpResponse::Ok())),
            )
            .default_service(|r: ServiceRequest| {
                ok(r.into_response(HttpResponse::MethodNotAllowed()))
            }),
    )
    .await;

    let req = TestRequest::with_uri("/blah").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    let req = TestRequest::with_uri("/test2").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/test2")
        .method(Method::POST)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[kayrx::test]
async fn test_data_factory() {
    let mut srv =
        init_service(App::new().data_factory(|| ok::<_, ()>(10usize)).service(
            web::resource("/").to(|_: web::Data<usize>| HttpResponse::Ok()),
        ))
        .await;
    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let mut srv =
        init_service(App::new().data_factory(|| ok::<_, ()>(10u32)).service(
            web::resource("/").to(|_: web::Data<usize>| HttpResponse::Ok()),
        ))
        .await;
    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[kayrx::test]
async fn test_extension() {
    let mut srv = init_service(App::new().app_data(10usize).service(
        web::resource("/").to(|req: HttpRequest| {
            assert_eq!(*req.app_data::<usize>().unwrap(), 10);
            HttpResponse::Ok()
        }),
    ))
    .await;
    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_wrap() {
    let mut srv = init_service(
        App::new()
            .wrap(
                DefaultHeaders::new()
                    .header(header::CONTENT_TYPE, HeaderValue::from_static("0001")),
            )
            .route("/test", web::get().to(|| HttpResponse::Ok())),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        HeaderValue::from_static("0001")
    );
}

#[kayrx::test]
async fn test_router_wrap() {
    let mut srv = init_service(
        App::new()
            .route("/test", web::get().to(|| HttpResponse::Ok()))
            .wrap(
                DefaultHeaders::new()
                    .header(header::CONTENT_TYPE, HeaderValue::from_static("0001")),
            ),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        HeaderValue::from_static("0001")
    );
}

#[kayrx::test]
async fn test_wrap_fn() {
    let mut srv = init_service(
        App::new()
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
            .service(web::resource("/test").to(|| HttpResponse::Ok())),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        HeaderValue::from_static("0001")
    );
}

#[kayrx::test]
async fn test_router_wrap_fn() {
    let mut srv = init_service(
        App::new()
            .route("/test", web::get().to(|| HttpResponse::Ok()))
            .wrap_fn(|req, srv| {
                let fut = srv.call(req);
                async {
                    let mut res = fut.await?;
                    res.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("0001"),
                    );
                    Ok(res)
                }
            }),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        HeaderValue::from_static("0001")
    );
}

#[kayrx::test]
async fn test_external_resource() {
    let mut srv = init_service(
        App::new()
            .external_resource("youtube", "https://youtube.com/watch/{video_id}")
            .route(
                "/test",
                web::get().to(|req: HttpRequest| {
                    HttpResponse::Ok().body(format!(
                        "{}",
                        req.url_for("youtube", &["12345"]).unwrap()
                    ))
                }),
            ),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body, Bytes::from_static(b"https://youtube.com/watch/12345"));
}
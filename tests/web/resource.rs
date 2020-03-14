use std::time::Duration;

use kayrx::timer::delay_for;
use kayrx::service::Service;
use futures::future::ok;

use kayrx::http::{header, HeaderValue, Method, StatusCode};
use kayrx::web::middleware::DefaultHeaders;
use kayrx::web::service::ServiceRequest;
use kayrx::web::test::{call_service, init_service, TestRequest};
use kayrx::web::{guard, self, App};
use kayrx::http::{Error, Response as HttpResponse};

#[kayrx::test]
async fn test_middleware() {
    let mut srv =
        init_service(
            App::new().service(
                web::resource("/test")
                    .name("test")
                    .wrap(DefaultHeaders::new().header(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("0001"),
                    ))
                    .route(web::get().to(|| HttpResponse::Ok())),
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
async fn test_middleware_fn() {
    let mut srv = init_service(
        App::new().service(
            web::resource("/test")
                .wrap_fn(|req, srv| {
                    let fut = srv.call(req);
                    async {
                        fut.await.map(|mut res| {
                            res.headers_mut().insert(
                                header::CONTENT_TYPE,
                                HeaderValue::from_static("0001"),
                            );
                            res
                        })
                    }
                })
                .route(web::get().to(|| HttpResponse::Ok())),
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
async fn test_to() {
    let mut srv =
        init_service(App::new().service(web::resource("/test").to(|| {
            async {
                delay_for(Duration::from_millis(100)).await;
                Ok::<_, Error>(HttpResponse::Ok())
            }
        })))
        .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_pattern() {
    let mut srv = init_service(
        App::new().service(
            web::resource(["/test", "/test2"])
                .to(|| async { Ok::<_, Error>(HttpResponse::Ok()) }),
        ),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let req = TestRequest::with_uri("/test2").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_default_resource() {
    let mut srv = init_service(
        App::new()
            .service(
                web::resource("/test").route(web::get().to(|| HttpResponse::Ok())),
            )
            .default_service(|r: ServiceRequest| {
                ok(r.into_response(HttpResponse::BadRequest()))
            }),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/test")
        .method(Method::POST)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    let mut srv = init_service(
        App::new().service(
            web::resource("/test")
                .route(web::get().to(|| HttpResponse::Ok()))
                .default_service(|r: ServiceRequest| {
                    ok(r.into_response(HttpResponse::BadRequest()))
                }),
        ),
    )
    .await;

    let req = TestRequest::with_uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/test")
        .method(Method::POST)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[kayrx::test]
async fn test_resource_guards() {
    let mut srv = init_service(
        App::new()
            .service(
                web::resource("/test/{p}")
                    .guard(guard::Get())
                    .to(|| HttpResponse::Ok()),
            )
            .service(
                web::resource("/test/{p}")
                    .guard(guard::Put())
                    .to(|| HttpResponse::Created()),
            )
            .service(
                web::resource("/test/{p}")
                    .guard(guard::Delete())
                    .to(|| HttpResponse::NoContent()),
            ),
    )
    .await;

    let req = TestRequest::with_uri("/test/it")
        .method(Method::GET)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/test/it")
        .method(Method::PUT)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = TestRequest::with_uri("/test/it")
        .method(Method::DELETE)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[kayrx::test]
async fn test_data() {
    let mut srv = init_service(
        App::new()
            .data(1.0f64)
            .data(1usize)
            .app_data(web::Data::new('-'))
            .service(
                web::resource("/test")
                    .data(10usize)
                    .app_data(web::Data::new('*'))
                    .guard(guard::Get())
                    .to(
                        |data1: web::Data<usize>,
                         data2: web::Data<char>,
                         data3: web::Data<f64>| {
                            assert_eq!(**data1, 10);
                            assert_eq!(**data2, '*');
                            assert_eq!(**data3, 1.0);
                            HttpResponse::Ok()
                        },
                    ),
            ),
    )
    .await;

    let req = TestRequest::get().uri("/test").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
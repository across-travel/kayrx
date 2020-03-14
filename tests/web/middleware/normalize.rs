use kayrx::service::{IntoService, Service, Transform};
use futures::future::ok;
use kayrx::web::dev::ServiceRequest;
use kayrx::web::test::{call_service, init_service, TestRequest};
use kayrx::web::{self, App};
use kayrx::http::Response as HttpResponse;
use kayrx::web::middleware::NormalizePath;

#[kayrx::test]
async fn test_wrap() {
    let mut app = init_service(
        App::new()
            .wrap(NormalizePath::default())
            .service(web::resource("/v1/something/").to(|| HttpResponse::Ok())),
    )
    .await;

    let req = TestRequest::with_uri("/v1//something////").to_request();
    let res = call_service(&mut app, req).await;
    assert!(res.status().is_success());
}

#[kayrx::test]
async fn test_in_place_normalization() {
    let srv = |req: ServiceRequest| {
        assert_eq!("/v1/something/", req.path());
        ok(req.into_response(HttpResponse::Ok().finish()))
    };

    let mut normalize = NormalizePath
        .new_transform(srv.into_service())
        .await
        .unwrap();

    let req = TestRequest::with_uri("/v1//something////").to_srv_request();
    let res = normalize.call(req).await.unwrap();
    assert!(res.status().is_success());
}

#[kayrx::test]
async fn should_normalize_nothing() {
    const URI: &str = "/v1/something/";

    let srv = |req: ServiceRequest| {
        assert_eq!(URI, req.path());
        ok(req.into_response(HttpResponse::Ok().finish()))
    };

    let mut normalize = NormalizePath
        .new_transform(srv.into_service())
        .await
        .unwrap();

    let req = TestRequest::with_uri(URI).to_srv_request();
    let res = normalize.call(req).await.unwrap();
    assert!(res.status().is_success());
}
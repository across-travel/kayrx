use kayrx::web::test::{init_service, TestRequest};
use kayrx::web::{guard, web, App};
use kayrx::http::{self, Response as HttpResponse};
use kayrx::service::Service;
use futures::future::ok;
use kayrx::web::dev::*;

#[test]
fn test_service_request() {
    let req = TestRequest::default().to_srv_request();
    let (r, pl) = req.into_parts();
    assert!(ServiceRequest::from_parts(r, pl).is_ok());

    let req = TestRequest::default().to_srv_request();
    let (r, pl) = req.into_parts();
    let _r2 = r.clone();
    assert!(ServiceRequest::from_parts(r, pl).is_err());

    let req = TestRequest::default().to_srv_request();
    let (r, _pl) = req.into_parts();
    assert!(ServiceRequest::from_request(r).is_ok());

    let req = TestRequest::default().to_srv_request();
    let (r, _pl) = req.into_parts();
    let _r2 = r.clone();
    assert!(ServiceRequest::from_request(r).is_err());
}

#[kayrx::test]
async fn test_service() {
    let mut srv = init_service(
        App::new().service(web::service("/test").name("test").finish(
            |req: ServiceRequest| ok(req.into_response(HttpResponse::Ok().finish())),
        )),
    )
    .await;
    let req = TestRequest::with_uri("/test").to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), http::StatusCode::OK);

    let mut srv = init_service(
        App::new().service(web::service("/test").guard(guard::Get()).finish(
            |req: ServiceRequest| ok(req.into_response(HttpResponse::Ok().finish())),
        )),
    )
    .await;
    let req = TestRequest::with_uri("/test")
        .method(http::Method::PUT)
        .to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
}

#[test]
fn test_fmt_debug() {
    let req = TestRequest::get()
        .uri("/index.html?test=1")
        .header("x-test", "111")
        .to_srv_request();
    let s = format!("{:?}", req);
    assert!(s.contains("ServiceRequest"));
    assert!(s.contains("test=1"));
    assert!(s.contains("x-test"));

    let res = HttpResponse::Ok().header("x-test", "111").finish();
    let res = TestRequest::post()
        .uri("/index.html?test=1")
        .to_srv_response(res);

    let s = format!("{:?}", res);
    assert!(s.contains("ServiceResponse"));
    assert!(s.contains("x-test"));
}
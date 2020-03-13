use kayrx::service::IntoService;
use kayrx::service::Service;
use futures::future::ok;
use kayrx::service::Transform;
use kayrx::web::dev::ServiceRequest;
use kayrx::http::header::CONTENT_TYPE;
use kayrx::web::test::{ok_service, TestRequest};
use kayrx::http::Response as HttpResponse;
use kayrx::web::middleware::DefaultHeaders;

#[kayrx::test]
async fn test_default_headers() {
    let mut mw = DefaultHeaders::new()
        .header(CONTENT_TYPE, "0001")
        .new_transform(ok_service())
        .await
        .unwrap();

    let req = TestRequest::default().to_srv_request();
    let resp = mw.call(req).await.unwrap();
    assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "0001");

    let req = TestRequest::default().to_srv_request();
    let srv = |req: ServiceRequest| {
        ok(req
            .into_response(HttpResponse::Ok().header(CONTENT_TYPE, "0002").finish()))
    };
    let mut mw = DefaultHeaders::new()
        .header(CONTENT_TYPE, "0001")
        .new_transform(srv.into_service())
        .await
        .unwrap();
    let resp = mw.call(req).await.unwrap();
    assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "0002");
}

#[kayrx::test]
async fn test_content_type() {
    let srv =
        |req: ServiceRequest| ok(req.into_response(HttpResponse::Ok().finish()));
    let mut mw = DefaultHeaders::new()
        .content_type()
        .new_transform(srv.into_service())
        .await
        .unwrap();

    let req = TestRequest::default().to_srv_request();
    let resp = mw.call(req).await.unwrap();
    assert_eq!(
        resp.headers().get(CONTENT_TYPE).unwrap(),
        "application/octet-stream"
    );
}
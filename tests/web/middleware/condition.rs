use kayrx::service::IntoService;
use kayrx::service::Transform;
use kayrx::web::dev::{ServiceRequest, ServiceResponse};
use kayrx::http::error::Result;
use kayrx::http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use kayrx::web::test::{self, TestRequest};
use kayrx::http::Response;
use kayrx::web::middleware::*;
use futures::future::ok;
use kayrx::web::middleware::errhandlers::*;


fn render_500<B>(mut res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    res.response_mut()
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("0001"));
    Ok(ErrorHandlerResponse::Response(res))
}

#[kayrx::test]
async fn test_handler_enabled() {
    let srv = |req: ServiceRequest| {
        ok(req.into_response(Response::InternalServerError().finish()))
    };

    let mw =
        ErrorHandlers::new().handler(StatusCode::INTERNAL_SERVER_ERROR, render_500);

    let mut mw = Condition::new(true, mw)
        .new_transform(srv.into_service())
        .await
        .unwrap();
    let resp =
        test::call_service(&mut mw, TestRequest::default().to_srv_request()).await;
    assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "0001");
}

#[kayrx::test]
async fn test_handler_disabled() {
    let srv = |req: ServiceRequest| {
        ok(req.into_response(Response::InternalServerError().finish()))
    };

    let mw =
        ErrorHandlers::new().handler(StatusCode::INTERNAL_SERVER_ERROR, render_500);

    let mut mw = Condition::new(false, mw)
        .new_transform(srv.into_service())
        .await
        .unwrap();

    let resp =
        test::call_service(&mut mw, TestRequest::default().to_srv_request()).await;
    assert_eq!(resp.headers().get(CONTENT_TYPE), None);
}
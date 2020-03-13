use kayrx::service::IntoService;
use kayrx::service::Transform;
use futures::future::{ok, FutureExt};
use kayrx::web::middleware::errhandlers::*;
use kayrx::http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use kayrx::web::test::{self, TestRequest};
use kayrx::http::Response as HttpResponse;
use kayrx::web::dev::{ServiceRequest, ServiceResponse};
use kayrx::http::error::Result;

fn render_500<B>(mut res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    res.response_mut()
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("0001"));
    Ok(ErrorHandlerResponse::Response(res))
}

#[kayrx::test]
async fn test_handler() {
    let srv = |req: ServiceRequest| {
        ok(req.into_response(HttpResponse::InternalServerError().finish()))
    };

    let mut mw = ErrorHandlers::new()
        .handler(StatusCode::INTERNAL_SERVER_ERROR, render_500)
        .new_transform(srv.into_service())
        .await
        .unwrap();

    let resp =
        test::call_service(&mut mw, TestRequest::default().to_srv_request()).await;
    assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "0001");
}

fn render_500_async<B: 'static>(
    mut res: ServiceResponse<B>,
) -> Result<ErrorHandlerResponse<B>> {
    res.response_mut()
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("0001"));
    Ok(ErrorHandlerResponse::Future(ok(res).boxed_local()))
}

#[kayrx::test]
async fn test_handler_async() {
    let srv = |req: ServiceRequest| {
        ok(req.into_response(HttpResponse::InternalServerError().finish()))
    };

    let mut mw = ErrorHandlers::new()
        .handler(StatusCode::INTERNAL_SERVER_ERROR, render_500_async)
        .new_transform(srv.into_service())
        .await
        .unwrap();

    let resp =
        test::call_service(&mut mw, TestRequest::default().to_srv_request()).await;
    assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "0001");
}
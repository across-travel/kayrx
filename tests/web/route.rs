use std::time::Duration;

use kayrx::timer::delay_for;
use bytes::Bytes;
use serde::Serialize;
use kayrx::http::{Method, StatusCode};
use kayrx::web::test::{call_service, init_service, read_body, TestRequest};
use kayrx::web::{error, web, App};
use kayrx::http::Response as HttpResponse;

#[derive(Serialize, PartialEq, Debug)]
struct MyObject {
    name: String,
}

#[kayrx::test]
async fn test_route() {
    let mut srv = init_service(
        App::new()
            .service(
                web::resource("/test")
                    .route(web::get().to(|| HttpResponse::Ok()))
                    .route(web::put().to(|| {
                        async {
                            Err::<HttpResponse, _>(error::ErrorBadRequest("err"))
                        }
                    }))
                    .route(web::post().to(|| {
                        async {
                            delay_for(Duration::from_millis(100)).await;
                            HttpResponse::Created()
                        }
                    }))
                    .route(web::delete().to(|| {
                        async {
                            delay_for(Duration::from_millis(100)).await;
                            Err::<HttpResponse, _>(error::ErrorBadRequest("err"))
                        }
                    })),
            )
            .service(web::resource("/json").route(web::get().to(|| {
                async {
                    delay_for(Duration::from_millis(25)).await;
                    web::Json(MyObject {
                        name: "test".to_string(),
                    })
                }
            }))),
    )
    .await;

    let req = TestRequest::with_uri("/test")
        .method(Method::GET)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = TestRequest::with_uri("/test")
        .method(Method::POST)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = TestRequest::with_uri("/test")
        .method(Method::PUT)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = TestRequest::with_uri("/test")
        .method(Method::DELETE)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = TestRequest::with_uri("/test")
        .method(Method::HEAD)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    let req = TestRequest::with_uri("/json").to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = read_body(resp).await;
    assert_eq!(body, Bytes::from_static(b"{\"name\":\"test\"}"));
}
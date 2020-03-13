use kayrx::service::Service;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use kayrx::web::web::*;
use kayrx::http::StatusCode;
use kayrx::web::test::{self, init_service, TestRequest};
use kayrx::web::{web, App};
use kayrx::http::Response as HttpResponse;

#[kayrx::test]
async fn test_data_extractor() {
    let mut srv = init_service(App::new().data("TEST".to_string()).service(
        web::resource("/").to(|data: web::Data<String>| {
            assert_eq!(data.to_lowercase(), "test");
            HttpResponse::Ok()
        }),
    ))
    .await;

    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let mut srv =
        init_service(App::new().data(10u32).service(
            web::resource("/").to(|_: web::Data<usize>| HttpResponse::Ok()),
        ))
        .await;
    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[kayrx::test]
async fn test_app_data_extractor() {
    let mut srv =
        init_service(App::new().app_data(Data::new(10usize)).service(
            web::resource("/").to(|_: web::Data<usize>| HttpResponse::Ok()),
        ))
        .await;

    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let mut srv =
        init_service(App::new().app_data(Data::new(10u32)).service(
            web::resource("/").to(|_: web::Data<usize>| HttpResponse::Ok()),
        ))
        .await;
    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[kayrx::test]
async fn test_route_data_extractor() {
    let mut srv =
        init_service(App::new().service(web::resource("/").data(10usize).route(
            web::get().to(|data: web::Data<usize>| {
                let _ = data.clone();
                HttpResponse::Ok()
            }),
        )))
        .await;

    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // different type
    let mut srv = init_service(
        App::new().service(
            web::resource("/")
                .data(10u32)
                .route(web::get().to(|_: web::Data<usize>| HttpResponse::Ok())),
        ),
    )
    .await;
    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[kayrx::test]
async fn test_override_data() {
    let mut srv = init_service(App::new().data(1usize).service(
        web::resource("/").data(10usize).route(web::get().to(
            |data: web::Data<usize>| {
                assert_eq!(**data, 10);
                let _ = data.clone();
                HttpResponse::Ok()
            },
        )),
    ))
    .await;

    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_data_drop() {
    struct TestData(Arc<AtomicUsize>);

    impl TestData {
        fn new(inner: Arc<AtomicUsize>) -> Self {
            let _ = inner.fetch_add(1, Ordering::SeqCst);
            Self(inner)
        }
    }

    impl Clone for TestData {
        fn clone(&self) -> Self {
            let inner = self.0.clone();
            let _ = inner.fetch_add(1, Ordering::SeqCst);
            Self(inner)
        }
    }

    impl Drop for TestData {
        fn drop(&mut self) {
            let _ = self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    let num = Arc::new(AtomicUsize::new(0));
    let data = TestData::new(num.clone());
    assert_eq!(num.load(Ordering::SeqCst), 1);

    let srv = test::start(move || {
        let data = data.clone();

        App::new()
            .data(data)
            .service(web::resource("/").to(|_data: Data<TestData>| async { "ok" }))
    });

    assert!(srv.get("/").send().await.unwrap().status().is_success());
    srv.stop().await;

    assert_eq!(num.load(Ordering::SeqCst), 0);
}
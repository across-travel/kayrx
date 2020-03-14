use kayrx::service::Service;
use bytes::Bytes;

use kayrx::web::*;
use kayrx::web::dev::*;
use kayrx::http::{Method, StatusCode};
use kayrx::web::test::{call_service, init_service, read_body, TestRequest};
use kayrx::web::{self, App, HttpRequest};
use kayrx::http::Response as HttpResponse;

#[kayrx::test]
async fn test_data() {
    let cfg = |cfg: &mut ServiceConfig| {
        cfg.data(10usize);
    };

    let mut srv =
        init_service(App::new().configure(cfg).service(
            web::resource("/").to(|_: web::Data<usize>| HttpResponse::Ok()),
        ))
        .await;
    let req = TestRequest::default().to_request();
    let resp = srv.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// #[kayrx::test]
// async fn test_data_factory() {
//     let cfg = |cfg: &mut ServiceConfig| {
//         cfg.data_factory(|| {
//             sleep(std::time::Duration::from_millis(50)).then(|_| {
//                 println!("READY");
//                 Ok::<_, ()>(10usize)
//             })
//         });
//     };

//     let mut srv =
//         init_service(App::new().configure(cfg).service(
//             web::resource("/").to(|_: web::Data<usize>| HttpResponse::Ok()),
//         ));
//     let req = TestRequest::default().to_request();
//     let resp = srv.call(req).await.unwrap();
//     assert_eq!(resp.status(), StatusCode::OK);

//     let cfg2 = |cfg: &mut ServiceConfig| {
//         cfg.data_factory(|| Ok::<_, ()>(10u32));
//     };
//     let mut srv = init_service(
//         App::new()
//             .service(web::resource("/").to(|_: web::Data<usize>| HttpResponse::Ok()))
//             .configure(cfg2),
//     );
//     let req = TestRequest::default().to_request();
//     let resp = srv.call(req).await.unwrap();
//     assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
// }

#[kayrx::test]
async fn test_external_resource() {
    let mut srv = init_service(
        App::new()
            .configure(|cfg| {
                cfg.external_resource(
                    "youtube",
                    "https://youtube.com/watch/{video_id}",
                );
            })
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

#[kayrx::test]
async fn test_service() {
    let mut srv = init_service(App::new().configure(|cfg| {
        cfg.service(
            web::resource("/test").route(web::get().to(|| HttpResponse::Created())),
        )
        .route("/index.html", web::get().to(|| HttpResponse::Ok()));
    }))
    .await;

    let req = TestRequest::with_uri("/test")
        .method(Method::GET)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = TestRequest::with_uri("/index.html")
        .method(Method::GET)
        .to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
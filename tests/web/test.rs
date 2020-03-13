use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use kayrx::http::httpmessage::HttpMessage;
use kayrx::web::{web, App, Responder};
use kayrx::http::{header, Version, error::Error};
use kayrx::http::Response as HttpResponse;
use kayrx::web::test::*;
use kayrx::web::web::*;
use kayrx::web::dev::*;

#[kayrx::test]
async fn test_basics() {
    let req = TestRequest::with_hdr(header::ContentType::json())
        .version(Version::HTTP_2)
        .set(header::Date(SystemTime::now().into()))
        .param("test", "123")
        .data(10u32)
        .app_data(20u64)
        .peer_addr("127.0.0.1:8081".parse().unwrap())
        .to_http_request();
    assert!(req.headers().contains_key(header::CONTENT_TYPE));
    assert!(req.headers().contains_key(header::DATE));
    assert_eq!(
        req.head().peer_addr,
        Some("127.0.0.1:8081".parse().unwrap())
    );
    assert_eq!(&req.match_info()["test"], "123");
    assert_eq!(req.version(), Version::HTTP_2);
    let data = req.app_data::<Data<u32>>().unwrap();
    assert!(req.app_data::<Data<u64>>().is_none());
    assert_eq!(*data.get_ref(), 10);

    assert!(req.app_data::<u32>().is_none());
    let data = req.app_data::<u64>().unwrap();
    assert_eq!(*data, 20);
}

#[kayrx::test]
async fn test_request_methods() {
    let mut app = init_service(
        App::new().service(
            web::resource("/index.html")
                .route(web::put().to(|| async { HttpResponse::Ok().body("put!") }))
                .route(
                    web::patch().to(|| async { HttpResponse::Ok().body("patch!") }),
                )
                .route(
                    web::delete()
                        .to(|| async { HttpResponse::Ok().body("delete!") }),
                ),
        ),
    )
    .await;

    let put_req = TestRequest::put()
        .uri("/index.html")
        .header(header::CONTENT_TYPE, "application/json")
        .to_request();

    let result = read_response(&mut app, put_req).await;
    assert_eq!(result, Bytes::from_static(b"put!"));

    let patch_req = TestRequest::patch()
        .uri("/index.html")
        .header(header::CONTENT_TYPE, "application/json")
        .to_request();

    let result = read_response(&mut app, patch_req).await;
    assert_eq!(result, Bytes::from_static(b"patch!"));

    let delete_req = TestRequest::delete().uri("/index.html").to_request();
    let result = read_response(&mut app, delete_req).await;
    assert_eq!(result, Bytes::from_static(b"delete!"));
}

#[kayrx::test]
async fn test_response() {
    let mut app =
        init_service(App::new().service(web::resource("/index.html").route(
            web::post().to(|| async { HttpResponse::Ok().body("welcome!") }),
        )))
        .await;

    let req = TestRequest::post()
        .uri("/index.html")
        .header(header::CONTENT_TYPE, "application/json")
        .to_request();

    let result = read_response(&mut app, req).await;
    assert_eq!(result, Bytes::from_static(b"welcome!"));
}

#[derive(Serialize, Deserialize)]
pub struct Person {
    id: String,
    name: String,
}

#[kayrx::test]
async fn test_response_json() {
    let mut app = init_service(App::new().service(web::resource("/people").route(
        web::post().to(|person: web::Json<Person>| {
            async { HttpResponse::Ok().json(person.into_inner()) }
        }),
    )))
    .await;

    let payload = r#"{"id":"12345","name":"User name"}"#.as_bytes();

    let req = TestRequest::post()
        .uri("/people")
        .header(header::CONTENT_TYPE, "application/json")
        .set_payload(payload)
        .to_request();

    let result: Person = read_response_json(&mut app, req).await;
    assert_eq!(&result.id, "12345");
}

#[kayrx::test]
async fn test_request_response_form() {
    let mut app = init_service(App::new().service(web::resource("/people").route(
        web::post().to(|person: web::Form<Person>| {
            async { HttpResponse::Ok().json(person.into_inner()) }
        }),
    )))
    .await;

    let payload = Person {
        id: "12345".to_string(),
        name: "User name".to_string(),
    };

    let req = TestRequest::post()
        .uri("/people")
        .set_form(&payload)
        .to_request();

    assert_eq!(req.content_type(), "application/x-www-form-urlencoded");

    let result: Person = read_response_json(&mut app, req).await;
    assert_eq!(&result.id, "12345");
    assert_eq!(&result.name, "User name");
}

#[kayrx::test]
async fn test_request_response_json() {
    let mut app = init_service(App::new().service(web::resource("/people").route(
        web::post().to(|person: web::Json<Person>| {
            async { HttpResponse::Ok().json(person.into_inner()) }
        }),
    )))
    .await;

    let payload = Person {
        id: "12345".to_string(),
        name: "User name".to_string(),
    };

    let req = TestRequest::post()
        .uri("/people")
        .set_json(&payload)
        .to_request();

    assert_eq!(req.content_type(), "application/json");

    let result: Person = read_response_json(&mut app, req).await;
    assert_eq!(&result.id, "12345");
    assert_eq!(&result.name, "User name");
}

#[kayrx::test]
async fn test_async_with_block() {
    async fn async_with_block() -> Result<HttpResponse, Error> {
        let res = web::block(move || Some(4usize).ok_or("wrong")).await;

        match res {
            Ok(value) => Ok(HttpResponse::Ok()
                .content_type("text/plain")
                .body(format!("Async with block value: {}", value))),
            Err(_) => panic!("Unexpected"),
        }
    }

    let mut app = init_service(
        App::new().service(web::resource("/index.html").to(async_with_block)),
    )
    .await;

    let req = TestRequest::post().uri("/index.html").to_request();
    let res = app.call(req).await.unwrap();
    assert!(res.status().is_success());
}

#[kayrx::test]
async fn test_server_data() {
    async fn handler(data: web::Data<usize>) -> impl Responder {
        assert_eq!(**data, 10);
        HttpResponse::Ok()
    }

    let mut app = init_service(
        App::new()
            .data(10usize)
            .service(web::resource("/index.html").to(handler)),
    )
    .await;

    let req = TestRequest::post().uri("/index.html").to_request();
    let res = app.call(req).await.unwrap();
    assert!(res.status().is_success());
}
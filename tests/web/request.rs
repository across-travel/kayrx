use kayrx::web::*;
use kayrx::web::types::*;
use kayrx::web::dev::*;
use std::rc::Rc;
use std::cell::RefCell;
use kayrx::web::dev::{ResourceDef, ResourceMap};
use kayrx::http::{header, StatusCode};
use kayrx::web::test::{call_service, init_service, TestRequest};
use kayrx::web::{self, App, error::*};
use kayrx::http::Response as HttpResponse;

#[test]
fn test_debug() {
    let req =
        TestRequest::with_header("content-type", "text/plain").to_http_request();
    let dbg = format!("{:?}", req);
    assert!(dbg.contains("HttpRequest"));
}

#[cfg(feature = "cookie")]
#[test]
fn test_no_request_cookies() {
    let req = TestRequest::default().to_http_request();
    assert!(req.cookies().unwrap().is_empty());
}

#[cfg(feature = "cookie")]
#[test]
fn test_request_cookies() {
    let req = TestRequest::default()
        .header(header::COOKIE, "cookie1=value1")
        .header(header::COOKIE, "cookie2=value2")
        .to_http_request();
    {
        let cookies = req.cookies().unwrap();
        assert_eq!(cookies.len(), 2);
        assert_eq!(cookies[0].name(), "cookie2");
        assert_eq!(cookies[0].value(), "value2");
        assert_eq!(cookies[1].name(), "cookie1");
        assert_eq!(cookies[1].value(), "value1");
    }

    let cookie = req.cookie("cookie1");
    assert!(cookie.is_some());
    let cookie = cookie.unwrap();
    assert_eq!(cookie.name(), "cookie1");
    assert_eq!(cookie.value(), "value1");

    let cookie = req.cookie("cookie-unknown");
    assert!(cookie.is_none());
}

#[test]
fn test_request_query() {
    let req = TestRequest::with_uri("/?id=test").to_http_request();
    assert_eq!(req.query_string(), "id=test");
}

#[test]
fn test_url_for() {
    let mut res = ResourceDef::new("/user/{name}.{ext}");
    *res.name_mut() = "index".to_string();

    let mut rmap = ResourceMap::new(ResourceDef::new(""));
    rmap.add(&mut res, None);
    assert!(rmap.has_resource("/user/test.html"));
    assert!(!rmap.has_resource("/test/unknown"));

    let req = TestRequest::with_header(header::HOST, "www.rust-lang.org")
        .rmap(rmap)
        .to_http_request();

    assert_eq!(
        req.url_for("unknown", &["test"]),
        Err(UrlGenerationError::ResourceNotFound)
    );
    assert_eq!(
        req.url_for("index", &["test"]),
        Err(UrlGenerationError::NotEnoughElements)
    );
    let url = req.url_for("index", &["test", "html"]);
    assert_eq!(
        url.ok().unwrap().as_str(),
        "http://www.rust-lang.org/user/test.html"
    );
}

#[test]
fn test_url_for_static() {
    let mut rdef = ResourceDef::new("/index.html");
    *rdef.name_mut() = "index".to_string();

    let mut rmap = ResourceMap::new(ResourceDef::new(""));
    rmap.add(&mut rdef, None);

    assert!(rmap.has_resource("/index.html"));

    let req = TestRequest::with_uri("/test")
        .header(header::HOST, "www.rust-lang.org")
        .rmap(rmap)
        .to_http_request();
    let url = req.url_for_static("index");
    assert_eq!(
        url.ok().unwrap().as_str(),
        "http://www.rust-lang.org/index.html"
    );
}

#[test]
fn test_url_for_external() {
    let mut rdef = ResourceDef::new("https://youtube.com/watch/{video_id}");

    *rdef.name_mut() = "youtube".to_string();

    let mut rmap = ResourceMap::new(ResourceDef::new(""));
    rmap.add(&mut rdef, None);
    assert!(rmap.has_resource("https://youtube.com/watch/unknown"));

    let req = TestRequest::default().rmap(rmap).to_http_request();
    let url = req.url_for("youtube", &["oHg5SJYRHA0"]);
    assert_eq!(
        url.ok().unwrap().as_str(),
        "https://youtube.com/watch/oHg5SJYRHA0"
    );
}

#[kayrx::test]
async fn test_data() {
    let mut srv = init_service(App::new().app_data(10usize).service(
        web::resource("/").to(|req: HttpRequest| {
            if req.app_data::<usize>().is_some() {
                HttpResponse::Ok()
            } else {
                HttpResponse::BadRequest()
            }
        }),
    ))
    .await;

    let req = TestRequest::default().to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let mut srv = init_service(App::new().app_data(10u32).service(
        web::resource("/").to(|req: HttpRequest| {
            if req.app_data::<usize>().is_some() {
                HttpResponse::Ok()
            } else {
                HttpResponse::BadRequest()
            }
        }),
    ))
    .await;

    let req = TestRequest::default().to_request();
    let resp = call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[kayrx::test]
async fn test_extensions_dropped() {
    struct Tracker {
        pub dropped: bool,
    }
    struct Foo {
        tracker: Rc<RefCell<Tracker>>,
    }
    impl Drop for Foo {
        fn drop(&mut self) {
            self.tracker.borrow_mut().dropped = true;
        }
    }

    let tracker = Rc::new(RefCell::new(Tracker { dropped: false }));
    {
        let tracker2 = Rc::clone(&tracker);
        let mut srv = init_service(App::new().data(10u32).service(
            web::resource("/").to(move |req: HttpRequest| {
                req.extensions_mut().insert(Foo {
                    tracker: Rc::clone(&tracker2),
                });
                HttpResponse::Ok()
            }),
        ))
        .await;

        let req = TestRequest::default().to_request();
        let resp = call_service(&mut srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    assert!(tracker.borrow().dropped);
}
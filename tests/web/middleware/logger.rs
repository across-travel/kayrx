use kayrx::service::{IntoService, Service, Transform};
use futures::future::ok;
use std::fmt::Formatter;
use time::OffsetDateTime;
use kayrx::http::{header, StatusCode};
use kayrx::web::test::TestRequest;
use kayrx::web::middleware::Logger;
use kayrx::web::dev::ServiceRequest;
use kayrx::web::middleware::dev::{Format, FormatDisplay};
use kayrx::http::Response as HttpResponse;

#[kayrx::test]
async fn test_logger() {
    let srv = |req: ServiceRequest| {
        ok(req.into_response(
            HttpResponse::build(StatusCode::OK)
                .header("X-Test", "ttt")
                .finish(),
        ))
    };
    let logger = Logger::new("%% %{User-Agent}i %{X-Test}o %{HOME}e %D test");

    let mut srv = logger.new_transform(srv.into_service()).await.unwrap();

    let req = TestRequest::with_header(
        header::USER_AGENT,
        header::HeaderValue::from_static("Kayrx-WEB"),
    )
    .to_srv_request();
    let _res = srv.call(req).await;
}

#[kayrx::test]
async fn test_url_path() {
    let mut format = Format::new("%T %U");
    let req = TestRequest::with_header(
        header::USER_AGENT,
        header::HeaderValue::from_static("Kayrx-WEB"),
    )
    .uri("/test/route/yeah")
    .to_srv_request();

    let now = OffsetDateTime::now();
    for unit in &mut format.0 {
        unit.render_request(now, &req);
    }

    let resp = HttpResponse::build(StatusCode::OK).force_close().finish();
    for unit in &mut format.0 {
        unit.render_response(&resp);
    }

    let render = |fmt: &mut Formatter<'_>| {
        for unit in &format.0 {
            unit.render(fmt, 1024, now)?;
        }
        Ok(())
    };
    let s = format!("{}", FormatDisplay(&render));
    println!("{}", s);
    assert!(s.contains("/test/route/yeah"));
}

#[kayrx::test]
async fn test_default_format() {
    let mut format = Format::default();

    let req = TestRequest::with_header(
        header::USER_AGENT,
        header::HeaderValue::from_static("Kayrx-WEB"),
    )
    .to_srv_request();

    let now = OffsetDateTime::now();
    for unit in &mut format.0 {
        unit.render_request(now, &req);
    }

    let resp = HttpResponse::build(StatusCode::OK).force_close().finish();
    for unit in &mut format.0 {
        unit.render_response(&resp);
    }

    let entry_time = OffsetDateTime::now();
    let render = |fmt: &mut Formatter<'_>| {
        for unit in &format.0 {
            unit.render(fmt, 1024, entry_time)?;
        }
        Ok(())
    };
    let s = format!("{}", FormatDisplay(&render));
    assert!(s.contains("GET / HTTP/1.1"));
    assert!(s.contains("200 1024"));
    assert!(s.contains("Kayrx-WEB"));
}

#[kayrx::test]
async fn test_request_time_format() {
    let mut format = Format::new("%t");
    let req = TestRequest::default().to_srv_request();

    let now = OffsetDateTime::now();
    for unit in &mut format.0 {
        unit.render_request(now, &req);
    }

    let resp = HttpResponse::build(StatusCode::OK).force_close().finish();
    for unit in &mut format.0 {
        unit.render_response(&resp);
    }

    let render = |fmt: &mut Formatter<'_>| {
        for unit in &format.0 {
            unit.render(fmt, 1024, now)?;
        }
        Ok(())
    };
    let s = format!("{}", FormatDisplay(&render));
    assert!(s.contains(&format!("{}", now.format("%Y-%m-%dT%H:%M:%S"))));
}
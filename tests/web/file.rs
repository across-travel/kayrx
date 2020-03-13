use std::fs::{self, File};
use std::ops::Add;
use std::time::{Duration, SystemTime};
use bytes::Bytes;
use std::path::PathBuf;
use std::iter::FromIterator;

use kayrx::web::guard;
use kayrx::http::header::{ self, ContentDisposition, DispositionParam, DispositionType };
use kayrx::http::{Response as HttpResponse,  Method, StatusCode};
use kayrx::web::middleware::Compress;
use kayrx::web::test::{self, TestRequest};
use kayrx::web::{web, App, Responder};
use kayrx::service::ServiceFactory;
use kayrx::web::file::*;
use futures::future::ok;
use kayrx::web::dev::ServiceRequest;

#[kayrx::test]
async fn test_file_extension_to_mime() {
    let m = file_extension_to_mime("jpg");
    assert_eq!(m, mime::IMAGE_JPEG);

    let m = file_extension_to_mime("invalid extension!!");
    assert_eq!(m, mime::APPLICATION_OCTET_STREAM);

    let m = file_extension_to_mime("");
    assert_eq!(m, mime::APPLICATION_OCTET_STREAM);
}

#[kayrx::test]
async fn test_if_modified_since_without_if_none_match() {
    let file = NamedFile::open("Cargo.toml").unwrap();
    let since =
        header::HttpDate::from(SystemTime::now().add(Duration::from_secs(60)));

    let req = TestRequest::default()
        .header(header::IF_MODIFIED_SINCE, since)
        .to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
}

#[kayrx::test]
async fn test_if_modified_since_with_if_none_match() {
    let file = NamedFile::open("Cargo.toml").unwrap();
    let since =
        header::HttpDate::from(SystemTime::now().add(Duration::from_secs(60)));

    let req = TestRequest::default()
        .header(header::IF_NONE_MATCH, "miss_etag")
        .header(header::IF_MODIFIED_SINCE, since)
        .to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_ne!(resp.status(), StatusCode::NOT_MODIFIED);
}

#[kayrx::test]
async fn test_named_file_text() {
    assert!(NamedFile::open("test--").is_err());
    let mut file = NamedFile::open("Cargo.toml").unwrap();
    {
        file.file();
        let _f: &File = &file;
    }
    {
        let _f: &mut File = &mut file;
    }

    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/x-toml"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "inline; filename=\"Cargo.toml\""
    );
}

#[kayrx::test]
async fn test_named_file_content_disposition() {
    assert!(NamedFile::open("test--").is_err());
    let mut file = NamedFile::open("Cargo.toml").unwrap();
    {
        file.file();
        let _f: &File = &file;
    }
    {
        let _f: &mut File = &mut file;
    }

    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(
        resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "inline; filename=\"Cargo.toml\""
    );

    let file = NamedFile::open("Cargo.toml")
        .unwrap()
        .disable_content_disposition();
    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert!(resp.headers().get(header::CONTENT_DISPOSITION).is_none());
}

#[kayrx::test]
async fn test_named_file_non_ascii_file_name() {
    let mut file =
        NamedFile::from_file(File::open("Cargo.toml").unwrap(), "貨物.toml")
            .unwrap();
    {
        file.file();
        let _f: &File = &file;
    }
    {
        let _f: &mut File = &mut file;
    }

    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/x-toml"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "inline; filename=\"貨物.toml\"; filename*=UTF-8''%E8%B2%A8%E7%89%A9.toml"
    );
}

#[kayrx::test]
async fn test_named_file_set_content_type() {
    let mut file = NamedFile::open("Cargo.toml")
        .unwrap()
        .set_content_type(mime::TEXT_XML);
    {
        file.file();
        let _f: &File = &file;
    }
    {
        let _f: &mut File = &mut file;
    }

    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/xml"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "inline; filename=\"Cargo.toml\""
    );
}

#[kayrx::test]
async fn test_named_file_image() {
    let mut file = NamedFile::open("tests/test.png").unwrap();
    {
        file.file();
        let _f: &File = &file;
    }
    {
        let _f: &mut File = &mut file;
    }

    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/png"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "inline; filename=\"test.png\""
    );
}

#[kayrx::test]
async fn test_named_file_image_attachment() {
    let cd = ContentDisposition {
        disposition: DispositionType::Attachment,
        parameters: vec![DispositionParam::Filename(String::from("test.png"))],
    };
    let mut file = NamedFile::open("tests/test.png")
        .unwrap()
        .set_content_disposition(cd);
    {
        file.file();
        let _f: &File = &file;
    }
    {
        let _f: &mut File = &mut file;
    }

    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/png"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "attachment; filename=\"test.png\""
    );
}

#[kayrx::test]
async fn test_named_file_binary() {
    let mut file = NamedFile::open("tests/test.binary").unwrap();
    {
        file.file();
        let _f: &File = &file;
    }
    {
        let _f: &mut File = &mut file;
    }

    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/octet-stream"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "attachment; filename=\"test.binary\""
    );
}

#[kayrx::test]
async fn test_named_file_status_code_text() {
    let mut file = NamedFile::open("Cargo.toml")
        .unwrap()
        .set_status_code(StatusCode::NOT_FOUND);
    {
        file.file();
        let _f: &File = &file;
    }
    {
        let _f: &mut File = &mut file;
    }

    let req = TestRequest::default().to_http_request();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/x-toml"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "inline; filename=\"Cargo.toml\""
    );
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[kayrx::test]
async fn test_mime_override() {
    fn all_attachment(_: &mime::Name) -> DispositionType {
        DispositionType::Attachment
    }

    let mut srv = test::init_service(
        App::new().service(
            Files::new("/", ".")
                .mime_override(all_attachment)
                .index_file("Cargo.toml"),
        ),
    )
    .await;

    let request = TestRequest::get().uri("/").to_request();
    let response = test::call_service(&mut srv, request).await;
    assert_eq!(response.status(), StatusCode::OK);

    let content_disposition = response
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .expect("To have CONTENT_DISPOSITION");
    let content_disposition = content_disposition
        .to_str()
        .expect("Convert CONTENT_DISPOSITION to str");
    assert_eq!(content_disposition, "attachment; filename=\"Cargo.toml\"");
}

#[kayrx::test]
async fn test_named_file_ranges_status_code() {
    let mut srv = test::init_service(
        App::new().service(Files::new("/test", ".").index_file("Cargo.toml")),
    )
    .await;

    // Valid range header
    let request = TestRequest::get()
        .uri("/t%65st/Cargo.toml")
        .header(header::RANGE, "bytes=10-20")
        .to_request();
    let response = test::call_service(&mut srv, request).await;
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);

    // Invalid range header
    let request = TestRequest::get()
        .uri("/t%65st/Cargo.toml")
        .header(header::RANGE, "bytes=1-0")
        .to_request();
    let response = test::call_service(&mut srv, request).await;

    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
}

#[kayrx::test]
async fn test_named_file_content_range_headers() {
    let mut srv = test::init_service(
        App::new().service(Files::new("/test", ".").index_file("tests/test.binary")),
    )
    .await;

    // Valid range header
    let request = TestRequest::get()
        .uri("/t%65st/tests/test.binary")
        .header(header::RANGE, "bytes=10-20")
        .to_request();

    let response = test::call_service(&mut srv, request).await;
    let contentrange = response
        .headers()
        .get(header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap();

    assert_eq!(contentrange, "bytes 10-20/100");

    // Invalid range header
    let request = TestRequest::get()
        .uri("/t%65st/tests/test.binary")
        .header(header::RANGE, "bytes=10-5")
        .to_request();
    let response = test::call_service(&mut srv, request).await;

    let contentrange = response
        .headers()
        .get(header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap();

    assert_eq!(contentrange, "bytes */100");
}

#[kayrx::test]
async fn test_named_file_content_length_headers() {
    // use kayrx::web::body::{MessageBody, ResponseBody};

    let mut srv = test::init_service(
        App::new().service(Files::new("test", ".").index_file("tests/test.binary")),
    )
    .await;

    // Valid range header
    let request = TestRequest::get()
        .uri("/t%65st/tests/test.binary")
        .header(header::RANGE, "bytes=10-20")
        .to_request();
    let _response = test::call_service(&mut srv, request).await;

    // let contentlength = response
    //     .headers()
    //     .get(header::CONTENT_LENGTH)
    //     .unwrap()
    //     .to_str()
    //     .unwrap();
    // assert_eq!(contentlength, "11");

    // Invalid range header
    let request = TestRequest::get()
        .uri("/t%65st/tests/test.binary")
        .header(header::RANGE, "bytes=10-8")
        .to_request();
    let response = test::call_service(&mut srv, request).await;
    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);

    // Without range header
    let request = TestRequest::get()
        .uri("/t%65st/tests/test.binary")
        // .no_default_headers()
        .to_request();
    let _response = test::call_service(&mut srv, request).await;

    // let contentlength = response
    //     .headers()
    //     .get(header::CONTENT_LENGTH)
    //     .unwrap()
    //     .to_str()
    //     .unwrap();
    // assert_eq!(contentlength, "100");

    // chunked
    let request = TestRequest::get()
        .uri("/t%65st/tests/test.binary")
        .to_request();
    let response = test::call_service(&mut srv, request).await;

    // with enabled compression
    // {
    //     let te = response
    //         .headers()
    //         .get(header::TRANSFER_ENCODING)
    //         .unwrap()
    //         .to_str()
    //         .unwrap();
    //     assert_eq!(te, "chunked");
    // }

    let bytes = test::read_body(response).await;
    let data = Bytes::from(fs::read("tests/test.binary").unwrap());
    assert_eq!(bytes, data);
}

#[kayrx::test]
async fn test_head_content_length_headers() {
    let mut srv = test::init_service(
        App::new().service(Files::new("test", ".").index_file("tests/test.binary")),
    )
    .await;

    // Valid range header
    let request = TestRequest::default()
        .method(Method::HEAD)
        .uri("/t%65st/tests/test.binary")
        .to_request();
    let _response = test::call_service(&mut srv, request).await;

    // TODO: fix check
    // let contentlength = response
    //     .headers()
    //     .get(header::CONTENT_LENGTH)
    //     .unwrap()
    //     .to_str()
    //     .unwrap();
    // assert_eq!(contentlength, "100");
}

#[kayrx::test]
async fn test_static_files_with_spaces() {
    let mut srv = test::init_service(
        App::new().service(Files::new("/", ".").index_file("Cargo.toml")),
    )
    .await;
    let request = TestRequest::get()
        .uri("/tests/test%20space.binary")
        .to_request();
    let response = test::call_service(&mut srv, request).await;
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = test::read_body(response).await;
    let data = Bytes::from(fs::read("tests/test space.binary").unwrap());
    assert_eq!(bytes, data);
}

#[kayrx::test]
async fn test_files_not_allowed() {
    let mut srv = test::init_service(App::new().service(Files::new("/", "."))).await;

    let req = TestRequest::default()
        .uri("/Cargo.toml")
        .method(Method::POST)
        .to_request();

    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    let mut srv = test::init_service(App::new().service(Files::new("/", "."))).await;
    let req = TestRequest::default()
        .method(Method::PUT)
        .uri("/Cargo.toml")
        .to_request();
    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[kayrx::test]
async fn test_files_guards() {
    let mut srv = test::init_service(
        App::new().service(Files::new("/", ".").use_guards(guard::Post())),
    )
    .await;

    let req = TestRequest::default()
        .uri("/Cargo.toml")
        .method(Method::POST)
        .to_request();

    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_named_file_content_encoding() {
    let mut srv = test::init_service(App::new().wrap(Compress::default()).service(
        web::resource("/").to(|| {
            async {
                NamedFile::open("Cargo.toml")
                    .unwrap()
                    .set_content_encoding(header::ContentEncoding::Identity)
            }
        }),
    ))
    .await;

    let request = TestRequest::get()
        .uri("/")
        .header(header::ACCEPT_ENCODING, "gzip")
        .to_request();
    let res = test::call_service(&mut srv, request).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!res.headers().contains_key(header::CONTENT_ENCODING));
}

#[kayrx::test]
async fn test_named_file_content_encoding_gzip() {
    let mut srv = test::init_service(App::new().wrap(Compress::default()).service(
        web::resource("/").to(|| {
            async {
                NamedFile::open("Cargo.toml")
                    .unwrap()
                    .set_content_encoding(header::ContentEncoding::Gzip)
            }
        }),
    ))
    .await;

    let request = TestRequest::get()
        .uri("/")
        .header(header::ACCEPT_ENCODING, "gzip")
        .to_request();
    let res = test::call_service(&mut srv, request).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::CONTENT_ENCODING)
            .unwrap()
            .to_str()
            .unwrap(),
        "gzip"
    );
}

#[kayrx::test]
async fn test_named_file_allowed_method() {
    let req = TestRequest::default().method(Method::GET).to_http_request();
    let file = NamedFile::open("Cargo.toml").unwrap();
    let resp = file.respond_to(&req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[kayrx::test]
async fn test_static_files() {
    let mut srv = test::init_service(
        App::new().service(Files::new("/", ".").show_files_listing()),
    )
    .await;
    let req = TestRequest::with_uri("/missing").to_request();

    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let mut srv = test::init_service(App::new().service(Files::new("/", "."))).await;

    let req = TestRequest::default().to_request();
    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let mut srv = test::init_service(
        App::new().service(Files::new("/", ".").show_files_listing()),
    )
    .await;
    let req = TestRequest::with_uri("/tests").to_request();
    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/html; charset=utf-8"
    );

    let bytes = test::read_body(resp).await;
    assert!(format!("{:?}", bytes).contains("/tests/test.png"));
}

#[kayrx::test]
async fn test_redirect_to_slash_directory() {
    // should not redirect if no index
    let mut srv = test::init_service(
        App::new().service(Files::new("/", ".").redirect_to_slash_directory()),
    )
    .await;
    let req = TestRequest::with_uri("/tests").to_request();
    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // should redirect if index present
    let mut srv = test::init_service(
        App::new().service(
            Files::new("/", ".")
                .index_file("test.png")
                .redirect_to_slash_directory(),
        ),
    )
    .await;
    let req = TestRequest::with_uri("/tests").to_request();
    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    // should not redirect if the path is wrong
    let req = TestRequest::with_uri("/not_existing").to_request();
    let resp = test::call_service(&mut srv, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[kayrx::test]
async fn test_static_files_bad_directory() {
    let _st: Files = Files::new("/", "missing");
    let _st: Files = Files::new("/", "Cargo.toml");
}

#[kayrx::test]
async fn test_default_handler_file_missing() {
    let mut st = Files::new("/", ".")
        .default_handler(|req: ServiceRequest| {
            ok(req.into_response(HttpResponse::Ok().body("default content")))
        })
        .new_service(())
        .await
        .unwrap();
    let req = TestRequest::with_uri("/missing").to_srv_request();

    let resp = test::call_service(&mut st, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = test::read_body(resp).await;
    assert_eq!(bytes, Bytes::from_static(b"default content"));
}

//     #[kayrx::test]
//     async fn test_serve_index() {
//         let st = Files::new(".").index_file("test.binary");
//         let req = TestRequest::default().uri("/tests").finish();

//         let resp = st.handle(&req).respond_to(&req).unwrap();
//         let resp = resp.as_msg();
//         assert_eq!(resp.status(), StatusCode::OK);
//         assert_eq!(
//             resp.headers()
//                 .get(header::CONTENT_TYPE)
//                 .expect("content type"),
//             "application/octet-stream"
//         );
//         assert_eq!(
//             resp.headers()
//                 .get(header::CONTENT_DISPOSITION)
//                 .expect("content disposition"),
//             "attachment; filename=\"test.binary\""
//         );

//         let req = TestRequest::default().uri("/tests/").finish();
//         let resp = st.handle(&req).respond_to(&req).unwrap();
//         let resp = resp.as_msg();
//         assert_eq!(resp.status(), StatusCode::OK);
//         assert_eq!(
//             resp.headers().get(header::CONTENT_TYPE).unwrap(),
//             "application/octet-stream"
//         );
//         assert_eq!(
//             resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
//             "attachment; filename=\"test.binary\""
//         );

//         // nonexistent index file
//         let req = TestRequest::default().uri("/tests/unknown").finish();
//         let resp = st.handle(&req).respond_to(&req).unwrap();
//         let resp = resp.as_msg();
//         assert_eq!(resp.status(), StatusCode::NOT_FOUND);

//         let req = TestRequest::default().uri("/tests/unknown/").finish();
//         let resp = st.handle(&req).respond_to(&req).unwrap();
//         let resp = resp.as_msg();
//         assert_eq!(resp.status(), StatusCode::NOT_FOUND);
//     }

//     #[kayrx::test]
//     async fn test_serve_index_nested() {
//         let st = Files::new(".").index_file("mod.rs");
//         let req = TestRequest::default().uri("/src/client").finish();
//         let resp = st.handle(&req).respond_to(&req).unwrap();
//         let resp = resp.as_msg();
//         assert_eq!(resp.status(), StatusCode::OK);
//         assert_eq!(
//             resp.headers().get(header::CONTENT_TYPE).unwrap(),
//             "text/x-rust"
//         );
//         assert_eq!(
//             resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
//             "inline; filename=\"mod.rs\""
//         );
//     }

//     #[kayrx::test]
//     fn integration_serve_index() {
//         let mut srv = test::TestServer::with_factory(|| {
//             App::new().handler(
//                 "test",
//                 Files::new(".").index_file("Cargo.toml"),
//             )
//         });

//         let request = srv.get().uri(srv.url("/test")).finish().unwrap();
//         let response = srv.execute(request.send()).unwrap();
//         assert_eq!(response.status(), StatusCode::OK);
//         let bytes = srv.execute(response.body()).unwrap();
//         let data = Bytes::from(fs::read("Cargo.toml").unwrap());
//         assert_eq!(bytes, data);

//         let request = srv.get().uri(srv.url("/test/")).finish().unwrap();
//         let response = srv.execute(request.send()).unwrap();
//         assert_eq!(response.status(), StatusCode::OK);
//         let bytes = srv.execute(response.body()).unwrap();
//         let data = Bytes::from(fs::read("Cargo.toml").unwrap());
//         assert_eq!(bytes, data);

//         // nonexistent index file
//         let request = srv.get().uri(srv.url("/test/unknown")).finish().unwrap();
//         let response = srv.execute(request.send()).unwrap();
//         assert_eq!(response.status(), StatusCode::NOT_FOUND);

//         let request = srv.get().uri(srv.url("/test/unknown/")).finish().unwrap();
//         let response = srv.execute(request.send()).unwrap();
//         assert_eq!(response.status(), StatusCode::NOT_FOUND);
//     }

//     #[kayrx::test]
//     fn integration_percent_encoded() {
//         let mut srv = test::TestServer::with_factory(|| {
//             App::new().handler(
//                 "test",
//                 Files::new(".").index_file("Cargo.toml"),
//             )
//         });

//         let request = srv
//             .get()
//             .uri(srv.url("/test/%43argo.toml"))
//             .finish()
//             .unwrap();
//         let response = srv.execute(request.send()).unwrap();
//         assert_eq!(response.status(), StatusCode::OK);
//     }

#[kayrx::test]
async fn test_path_buf() {
    assert_eq!(
        PathBufWrp::get_pathbuf("/test/.tt").map(|t| t.0),
        Err(UriSegmentError::BadStart('.'))
    );
    assert_eq!(
        PathBufWrp::get_pathbuf("/test/*tt").map(|t| t.0),
        Err(UriSegmentError::BadStart('*'))
    );
    assert_eq!(
        PathBufWrp::get_pathbuf("/test/tt:").map(|t| t.0),
        Err(UriSegmentError::BadEnd(':'))
    );
    assert_eq!(
        PathBufWrp::get_pathbuf("/test/tt<").map(|t| t.0),
        Err(UriSegmentError::BadEnd('<'))
    );
    assert_eq!(
        PathBufWrp::get_pathbuf("/test/tt>").map(|t| t.0),
        Err(UriSegmentError::BadEnd('>'))
    );
    assert_eq!(
        PathBufWrp::get_pathbuf("/seg1/seg2/").unwrap().0,
        PathBuf::from_iter(vec!["seg1", "seg2"])
    );
    assert_eq!(
        PathBufWrp::get_pathbuf("/seg1/../seg2/").unwrap().0,
        PathBuf::from_iter(vec!["seg2"])
    );
}
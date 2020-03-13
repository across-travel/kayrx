use kayrx::web::client::Client;
use kayrx::http::header;

#[kayrx::test]
async fn test_debug() {
    let request = Client::new().ws("/").header("x-test", "111");
    let repr = format!("{:?}", request);
    assert!(repr.contains("WebsocketsRequest"));
    assert!(repr.contains("x-test"));
}

#[kayrx::test]
async fn test_header_override() {
    let req = Client::build()
        .header(header::CONTENT_TYPE, "111")
        .finish()
        .ws("/")
        .set_header(header::CONTENT_TYPE, "222");

    assert_eq!(
        req.head
            .headers
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap(),
        "222"
    );
}

#[kayrx::test]
async fn basic_auth() {
    let req = Client::new()
        .ws("/")
        .basic_auth("username", Some("password"));
    assert_eq!(
        req.head
            .headers
            .get(header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap(),
        "Basic dXNlcm5hbWU6cGFzc3dvcmQ="
    );

    let req = Client::new().ws("/").basic_auth("username", None);
    assert_eq!(
        req.head
            .headers
            .get(header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap(),
        "Basic dXNlcm5hbWU6"
    );
}

#[kayrx::test]
async fn bearer_auth() {
    let req = Client::new().ws("/").bearer_auth("someS3cr3tAutht0k3n");
    assert_eq!(
        req.head
            .headers
            .get(header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer someS3cr3tAutht0k3n"
    );
    let _ = req.connect();
}

#[cfg(feature = "cookie")]
#[kayrx::test]
async fn basics() {
    let req = Client::new()
        .ws("http://localhost/")
        .origin("test-origin")
        .max_frame_size(100)
        .server_mode()
        .protocols(&["v1", "v2"])
        .set_header_if_none(header::CONTENT_TYPE, "json")
        .set_header_if_none(header::CONTENT_TYPE, "text")
        .cookie(Cookie::build("cookie1", "value1").finish());
    assert_eq!(
        req.origin.as_ref().unwrap().to_str().unwrap(),
        "test-origin"
    );
    assert_eq!(req.max_size, 100);
    assert_eq!(req.server_mode, true);
    assert_eq!(req.protocols, Some("v1,v2".to_string()));
    assert_eq!(
        req.head.headers.get(header::CONTENT_TYPE).unwrap(),
        header::HeaderValue::from_static("json")
    );

    let _ = req.connect().await;

    assert!(Client::new().ws("/").connect().await.is_err());
    assert!(Client::new().ws("http:///test").connect().await.is_err());
    assert!(Client::new().ws("hmm://test.com/").connect().await.is_err());
}
//! Various helpers for kayrx applications to use during testing.
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::mpsc;
use std::{fmt, net, thread, time};
use bytes::{Bytes, BytesMut};
use futures_util::future::ok;
use futures_util::StreamExt;
use futures_core::stream::Stream;
use net2::TcpBuilder;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
#[cfg(feature = "cookie")]
use coo_kie::Cookie;

use crate::krse::io::{AsyncRead, AsyncWrite};
use crate::codec::Framed2 as Framed;
use crate::http::header::{ContentType, Header, HeaderName, IntoHeaderValue};	
use crate::http::{error::HttpError, Method, StatusCode, Uri, Version};
use crate::http::test::TestRequest as HttpTestRequest;
use crate::http::{Extensions, HttpService, Request};
use crate::websocket;
use crate::router::{Path, ResourceDef, Url};
use crate::{timer::delay_for, fiber::System};
use crate::service::{
    map_config, IntoService, IntoServiceFactory, Service, ServiceFactory,
};
use crate::web::client::error::PayloadError;
use crate::web::client::{Client, ClientRequest, ClientResponse, Connector};
use crate::web::config::AppConfig;
use crate::web::data::Data;
use crate::web::dev::{Body, MessageBody, Payload, Server};
use crate::web::request::HttpRequestPool;
use crate::web::rmap::ResourceMap;
use crate::web::service::{ServiceRequest, ServiceResponse};
use crate::http::{error::Error, Response as HttpResponse};
use crate::web::HttpRequest;

/// Create service that always responds with `HttpResponse::Ok()`
pub fn ok_service(
) -> impl Service<Request = ServiceRequest, Response = ServiceResponse<Body>, Error = Error>
{
    default_service(StatusCode::OK)
}

/// Create service that responds with response with specified status code
pub fn default_service(
    status_code: StatusCode,
) -> impl Service<Request = ServiceRequest, Response = ServiceResponse<Body>, Error = Error>
{
    (move |req: ServiceRequest| {
        ok(req.into_response(HttpResponse::build(status_code).finish()))
    })
    .into_service()
}

/// This method accepts application builder instance, and constructs
/// service.
///
/// ```rust
/// use kayrx::service::Service;
/// use kayrx::web::{test, self, App, HttpResponse};
/// use kayrx::http::StatusCode;
///
/// #[kayrx::test]
/// async fn test_init_service() {
///     let mut app = test::init_service(
///         App::new()
///             .service(web::resource("/test").to(|| async { HttpResponse::Ok() }))
///     ).await;
///
///     // Create request object
///     let req = test::TestRequest::with_uri("/test").to_request();
///
///     // Execute application
///     let resp = app.call(req).await.unwrap();
///     assert_eq!(resp.status(), StatusCode::OK);
/// }
/// ```
pub async fn init_service<R, S, B, E>(
    app: R,
) -> impl Service<Request = Request, Response = ServiceResponse<B>, Error = E>
where
    R: IntoServiceFactory<S>,
    S: ServiceFactory<
        Config = AppConfig,
        Request = Request,
        Response = ServiceResponse<B>,
        Error = E,
    >,
    S::InitError: std::fmt::Debug,
{
    let srv = app.into_factory();
    srv.new_service(AppConfig::default()).await.unwrap()
}

/// Calls service and waits for response future completion.
///
/// ```rust
/// use kayrx::web::{self, test, App, HttpResponse};
/// use kayrx::http::StatusCode;
/// use kayrx::service::Service;
///
/// #[test]
/// fn test_response() {
///     let mut app = test::init_service(
///         App::new()
///             .service(web::resource("/test").to(|| async {
///                 HttpResponse::Ok()
///             }))
///     ).await;
///
///     // Create request object
///     let req = test::TestRequest::with_uri("/test").to_request();
///
///     // Call application
///     let resp = test::call_service(&mut app, req).await;
///     assert_eq!(resp.status(), StatusCode::OK);
/// }
/// ```
pub async fn call_service<S, R, B, E>(app: &mut S, req: R) -> S::Response
where
    S: Service<Request = R, Response = ServiceResponse<B>, Error = E>,
    E: std::fmt::Debug,
{
    app.call(req).await.unwrap()
}

/// Helper function that returns a response body of a TestRequest
///
/// ```rust
/// use kayrx::web::{test, self, App, HttpResponse};
/// use kayrx::http::header;
/// use bytes::Bytes;
///
/// #[kayrx::test]
/// async fn test_index() {
///     let mut app = test::init_service(
///         App::new().service(
///             web::resource("/index.html")
///                 .route(web::post().to(|| async {
///                     HttpResponse::Ok().body("welcome!")
///                 })))
///     ).await;
///
///     let req = test::TestRequest::post()
///         .uri("/index.html")
///         .header(header::CONTENT_TYPE, "application/json")
///         .to_request();
///
///     let result = test::read_response(&mut app, req).await;
///     assert_eq!(result, Bytes::from_static(b"welcome!"));
/// }
/// ```
pub async fn read_response<S, B>(app: &mut S, req: Request) -> Bytes
where
    S: Service<Request = Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let mut resp = app
        .call(req)
        .await
        .unwrap_or_else(|_| panic!("read_response failed at application call"));

    let mut body = resp.take_body();
    let mut bytes = BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item.unwrap());
    }
    bytes.freeze()
}

/// Helper function that returns a response body of a ServiceResponse.
///
/// ```rust
/// use kayrx::web::{test, self, App, HttpResponse};
/// use kayrx::http::header;
/// use bytes::Bytes;
///
/// #[kayrx::test]
/// async fn test_index() {
///     let mut app = test::init_service(
///         App::new().service(
///             web::resource("/index.html")
///                 .route(web::post().to(|| async {
///                     HttpResponse::Ok().body("welcome!")
///                 })))
///     ).await;
///
///     let req = test::TestRequest::post()
///         .uri("/index.html")
///         .header(header::CONTENT_TYPE, "application/json")
///         .to_request();
///
///     let resp = test::call_service(&mut app, req).await;
///     let result = test::read_body(resp);
///     assert_eq!(result, Bytes::from_static(b"welcome!"));
/// }
/// ```
pub async fn read_body<B>(mut res: ServiceResponse<B>) -> Bytes
where
    B: MessageBody,
{
    let mut body = res.take_body();
    let mut bytes = BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item.unwrap());
    }
    bytes.freeze()
}

pub async fn load_stream<S>(mut stream: S) -> Result<Bytes, Error>
where
    S: Stream<Item = Result<Bytes, Error>> + Unpin,
{
    let mut data = BytesMut::new();
    while let Some(item) = stream.next().await {
        data.extend_from_slice(&item?);
    }
    Ok(data.freeze())
}

/// Helper function that returns a deserialized response body of a TestRequest
///
/// ```rust
/// use kayrx::web::{App, test, self, types, HttpResponse};
/// use kayrx::http::header;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// pub struct Person {
///     id: String,
///     name: String
/// }
///
/// #[kayrx::test]
/// async fn test_add_person() {
///     let mut app = test::init_service(
///         App::new().service(
///             web::resource("/people")
///                 .route(web::post().to(|person: types::Json<Person>| async {
///                     HttpResponse::Ok()
///                         .json(person.into_inner())})
///                     ))
///     ).await;
///
///     let payload = r#"{"id":"12345","name":"User name"}"#.as_bytes();
///
///     let req = test::TestRequest::post()
///         .uri("/people")
///         .header(header::CONTENT_TYPE, "application/json")
///         .set_payload(payload)
///         .to_request();
///
///     let result: Person = test::read_response_json(&mut app, req).await;
/// }
/// ```
pub async fn read_response_json<S, B, T>(app: &mut S, req: Request) -> T
where
    S: Service<Request = Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
    T: DeserializeOwned,
{
    let body = read_response(app, req).await;

    serde_json::from_slice(&body)
        .unwrap_or_else(|_| panic!("read_response_json failed during deserialization"))
}

/// Test `Request` builder.
///
/// For unit testing, kayrx provides a request builder type and a simple handler runner. TestRequest implements a builder-like pattern.
/// You can generate various types of request via TestRequest's methods:
///  * `TestRequest::to_request` creates `crate::http::Request` instance.
///  * `TestRequest::to_srv_request` creates `ServiceRequest` instance, which is used for testing middlewares and chain adapters.
///  * `TestRequest::to_srv_response` creates `ServiceResponse` instance.
///  * `TestRequest::to_http_request` creates `HttpRequest` instance, which is used for testing handlers.
///
/// ```rust
/// use kayrx::web::{test, HttpRequest, HttpResponse, HttpMessage};
/// use kayrx::http::{header, StatusCode};
///
/// async fn index(req: HttpRequest) -> HttpResponse {
///     if let Some(hdr) = req.headers().get(header::CONTENT_TYPE) {
///         HttpResponse::Ok().into()
///     } else {
///         HttpResponse::BadRequest().into()
///     }
/// }
///
/// #[test]
/// fn test_index() {
///     let req = test::TestRequest::with_header("content-type", "text/plain")
///         .to_http_request();
///
///     let resp = index(req).await.unwrap();
///     assert_eq!(resp.status(), StatusCode::OK);
///
///     let req = test::TestRequest::default().to_http_request();
///     let resp = index(req).await.unwrap();
///     assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
/// }
/// ```
pub struct TestRequest {
    req: HttpTestRequest,
    rmap: ResourceMap,
    config: AppConfig,
    path: Path<Url>,
    peer_addr: Option<SocketAddr>,
    app_data: Extensions,
}

impl Default for TestRequest {
    fn default() -> TestRequest {
        TestRequest {
            req: HttpTestRequest::default(),
            rmap: ResourceMap::new(ResourceDef::new("")),
            config: AppConfig::default(),
            path: Path::new(Url::new(Uri::default())),
            peer_addr: None,
            app_data: Extensions::new(),
        }
    }
}

#[allow(clippy::wrong_self_convention)]
impl TestRequest {
    /// Create TestRequest and set request uri
    pub fn with_uri(path: &str) -> TestRequest {
        TestRequest::default().uri(path)
    }

    /// Create TestRequest and set header	
    pub fn with_hdr<H: Header>(hdr: H) -> TestRequest {	
        TestRequest::default().set(hdr)	
    }

    /// Create TestRequest and set header
    pub fn with_header<K, V>(key: K, value: V) -> TestRequest
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<HttpError>,
        V: IntoHeaderValue,
    {
        TestRequest::default().header(key, value)
    }

    /// Create TestRequest and set method to `Method::GET`
    pub fn get() -> TestRequest {
        TestRequest::default().method(Method::GET)
    }

    /// Create TestRequest and set method to `Method::POST`
    pub fn post() -> TestRequest {
        TestRequest::default().method(Method::POST)
    }

    /// Create TestRequest and set method to `Method::PUT`
    pub fn put() -> TestRequest {
        TestRequest::default().method(Method::PUT)
    }

    /// Create TestRequest and set method to `Method::PATCH`
    pub fn patch() -> TestRequest {
        TestRequest::default().method(Method::PATCH)
    }

    /// Create TestRequest and set method to `Method::DELETE`
    pub fn delete() -> TestRequest {
        TestRequest::default().method(Method::DELETE)
    }

    /// Set HTTP version of this request
    pub fn version(mut self, ver: Version) -> Self {
        self.req.version(ver);
        self
    }

    /// Set HTTP method of this request
    pub fn method(mut self, meth: Method) -> Self {
        self.req.method(meth);
        self
    }

    /// Set HTTP Uri of this request
    pub fn uri(mut self, path: &str) -> Self {
        self.req.uri(path);
        self
    }
    
     /// Set a header	
     pub fn set<H: Header>(mut self, hdr: H) -> Self {	
        self.req.set(hdr);	
        self	
    }
    
    /// Set a header
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<HttpError>,
        V: IntoHeaderValue,
    {
        self.req.header(key, value);
        self
    }

    #[cfg(feature = "cookie")]
    /// Set cookie for this request
    pub fn cookie(mut self, cookie: Cookie<'_>) -> Self {
        self.req.cookie(cookie);
        self
    }

    /// Set request path pattern parameter
    pub fn param(mut self, name: &'static str, value: &'static str) -> Self {
        self.path.add_static(name, value);
        self
    }

    /// Set peer addr
    pub fn peer_addr(mut self, addr: SocketAddr) -> Self {
        self.peer_addr = Some(addr);
        self
    }

    /// Set request payload
    pub fn set_payload<B: Into<Bytes>>(mut self, data: B) -> Self {
        self.req.set_payload(data);
        self
    }

    /// Serialize `data` to a URL encoded form and set it as the request payload. The `Content-Type`
    /// header is set to `application/x-www-form-urlencoded`.
    pub fn set_form<T: Serialize>(mut self, data: &T) -> Self {
        let bytes = serde_urlencoded::to_string(data)
            .expect("Failed to serialize test data as a urlencoded form");
        self.req.set_payload(bytes);
        self.req.set(ContentType::form_url_encoded());
        self
    }

    /// Serialize `data` to JSON and set it as the request payload. The `Content-Type` header is
    /// set to `application/json`.
    pub fn set_json<T: Serialize>(mut self, data: &T) -> Self {
        let bytes =
            serde_json::to_string(data).expect("Failed to serialize test data to json");
        self.req.set_payload(bytes);
        self.req.set(ContentType::json());
        self
    }

    /// Set application data. This is equivalent of `App::data()` method
    /// for testing purpose.
    pub fn data<T: 'static>(mut self, data: T) -> Self {
        self.app_data.insert(Data::new(data));
        self
    }

    /// Set application data. This is equivalent of `App::app_data()` method
    /// for testing purpose.
    pub fn app_data<T: 'static>(mut self, data: T) -> Self {
        self.app_data.insert(data);
        self
    }

    /// TEST
    /// Set request config
    pub fn rmap(mut self, rmap: ResourceMap) -> Self {
        self.rmap = rmap;
        self
    }

    /// Complete request creation and generate `Request` instance
    pub fn to_request(mut self) -> Request {
        let mut req = self.req.finish();
        req.head_mut().peer_addr = self.peer_addr;
        req
    }

    /// Complete request creation and generate `ServiceRequest` instance
    pub fn to_srv_request(mut self) -> ServiceRequest {
        let (mut head, payload) = self.req.finish().into_parts();
        head.peer_addr = self.peer_addr;
        self.path.get_mut().update(&head.uri);

        ServiceRequest::new(HttpRequest::new(
            self.path,
            head,
            payload,
            Rc::new(self.rmap),
            self.config.clone(),
            Rc::new(self.app_data),
            HttpRequestPool::create(),
        ))
    }

    /// Complete request creation and generate `ServiceResponse` instance
    pub fn to_srv_response<B>(self, res: HttpResponse<B>) -> ServiceResponse<B> {
        self.to_srv_request().into_response(res)
    }

    /// Complete request creation and generate `HttpRequest` instance
    pub fn to_http_request(mut self) -> HttpRequest {
        let (mut head, payload) = self.req.finish().into_parts();
        head.peer_addr = self.peer_addr;
        self.path.get_mut().update(&head.uri);

        HttpRequest::new(
            self.path,
            head,
            payload,
            Rc::new(self.rmap),
            self.config.clone(),
            Rc::new(self.app_data),
            HttpRequestPool::create(),
        )
    }

    /// Complete request creation and generate `HttpRequest` and `Payload` instances
    pub fn to_http_parts(mut self) -> (HttpRequest, Payload) {
        let (mut head, payload) = self.req.finish().into_parts();
        head.peer_addr = self.peer_addr;
        self.path.get_mut().update(&head.uri);

        let req = HttpRequest::new(
            self.path,
            head,
            Payload::None,
            Rc::new(self.rmap),
            self.config.clone(),
            Rc::new(self.app_data),
            HttpRequestPool::create(),
        );

        (req, payload)
    }
}

/// Start test server with default configuration
///
/// Test server is very simple server that simplify process of writing
/// integration tests cases for kayrx web applications.
///
/// # Examples
///
/// ```rust
/// use kayrx::web::{self, test, App, HttpResponse, Error};
///
/// async fn my_handler() -> Result<HttpResponse, Error> {
///     Ok(HttpResponse::Ok().into())
/// }
///
/// #[kayrx::test]
/// async fn test_example() {
///     let mut srv = test::start(
///         || App::new().service(
///                 web::resource("/").to(my_handler))
///     );
///
///     let req = srv.get("/");
///     let response = req.send().await.unwrap();
///     assert!(response.status().is_success());
/// }
/// ```
pub fn start<F, I, S, B>(factory: F) -> TestServer
where
    F: Fn() -> I + Send + Clone + 'static,
    I: IntoServiceFactory<S>,
    S: ServiceFactory<Config = AppConfig, Request = Request> + 'static,
    S::Error: Into<Error> + 'static,
    S::InitError: fmt::Debug,
    S::Response: Into<HttpResponse<B>> + 'static,
    <S::Service as Service>::Future: 'static,
    B: MessageBody + 'static,
{
    start_with(TestServerConfig::default(), factory)
}

/// Start test server with custom configuration
///
/// Test server could be configured in different ways, for details check
/// `TestServerConfig` docs.
///
/// # Examples
///
/// ```rust
/// use kayrx::web::{self, test, App, HttpResponse, Error};
///
/// async fn my_handler() -> Result<HttpResponse, Error> {
///     Ok(HttpResponse::Ok().into())
/// }
///
/// #[kayrx::test]
/// async fn test_example() {
///     let mut srv = test::start_with(test::config().h1(), ||
///         App::new().service(web::resource("/").to(my_handler))
///     );
///
///     let req = srv.get("/");
///     let response = req.send().await.unwrap();
///     assert!(response.status().is_success());
/// }
/// ```
pub fn start_with<F, I, S, B>(cfg: TestServerConfig, factory: F) -> TestServer
where
    F: Fn() -> I + Send + Clone + 'static,
    I: IntoServiceFactory<S>,
    S: ServiceFactory<Config = AppConfig, Request = Request> + 'static,
    S::Error: Into<Error> + 'static,
    S::InitError: fmt::Debug,
    S::Response: Into<HttpResponse<B>> + 'static,
    <S::Service as Service>::Future: 'static,
    B: MessageBody + 'static,
{
    let (tx, rx) = mpsc::channel();

    let ssl = match cfg.stream {
        StreamType::Tcp => false,
        StreamType::Rustls(_) => true,
    };

    // run server in separate thread
    thread::spawn(move || {
        let sys = System::new("kayrx-test-server");
        let tcp = net::TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = tcp.local_addr().unwrap();
        let factory = factory.clone();
        let cfg = cfg.clone();
        let ctimeout = cfg.client_timeout;
        let builder = Server::build().workers(1).disable_signals();

        let srv = match cfg.stream {
            StreamType::Tcp => match cfg.tp {
                HttpVer::Http1 => builder.listen("test", tcp, move || {
                    let cfg =
                        AppConfig::new(false, local_addr, format!("{}", local_addr));
                    HttpService::build()
                        .client_timeout(ctimeout)
                        .h1(map_config(factory(), move |_| cfg.clone()))
                        .tcp()
                }),
                HttpVer::Http2 => builder.listen("test", tcp, move || {
                    let cfg =
                        AppConfig::new(false, local_addr, format!("{}", local_addr));
                    HttpService::build()
                        .client_timeout(ctimeout)
                        .h2(map_config(factory(), move |_| cfg.clone()))
                        .tcp()
                }),
                HttpVer::Both => builder.listen("test", tcp, move || {
                    let cfg =
                        AppConfig::new(false, local_addr, format!("{}", local_addr));
                    HttpService::build()
                        .client_timeout(ctimeout)
                        .finish(map_config(factory(), move |_| cfg.clone()))
                        .tcp()
                }),
            },

            StreamType::Rustls(config) => match cfg.tp {
                HttpVer::Http1 => builder.listen("test", tcp, move || {
                    let cfg =
                        AppConfig::new(true, local_addr, format!("{}", local_addr));
                    HttpService::build()
                        .client_timeout(ctimeout)
                        .h1(map_config(factory(), move |_| cfg.clone()))
                        .rustls(config.clone())
                }),
                HttpVer::Http2 => builder.listen("test", tcp, move || {
                    let cfg =
                        AppConfig::new(true, local_addr, format!("{}", local_addr));
                    HttpService::build()
                        .client_timeout(ctimeout)
                        .h2(map_config(factory(), move |_| cfg.clone()))
                        .rustls(config.clone())
                }),
                HttpVer::Both => builder.listen("test", tcp, move || {
                    let cfg =
                        AppConfig::new(true, local_addr, format!("{}", local_addr));
                    HttpService::build()
                        .client_timeout(ctimeout)
                        .finish(map_config(factory(), move |_| cfg.clone()))
                        .rustls(config.clone())
                }),
            },
        }
        .unwrap()
        .start();

        tx.send((System::current(), srv, local_addr)).unwrap();
        sys.run()
    });

    let (system, server, addr) = rx.recv().unwrap();

    let client = {
        let connector = {
                Connector::new()
                    .conn_lifetime(time::Duration::from_secs(0))
                    .timeout(time::Duration::from_millis(30000))
                    .finish()
        };

        Client::build().connector(connector).finish()
    };

    TestServer {
        ssl,
        addr,
        client,
        system,
        server,
    }
}

#[derive(Clone)]
pub struct TestServerConfig {
    tp: HttpVer,
    stream: StreamType,
    client_timeout: u64,
}

#[derive(Clone)]
enum HttpVer {
    Http1,
    Http2,
    Both,
}

#[derive(Clone)]
enum StreamType {
    Tcp,
    Rustls(rust_tls::ServerConfig),
}

impl Default for TestServerConfig {
    fn default() -> Self {
        TestServerConfig::new()
    }
}

/// Create default test server config
pub fn config() -> TestServerConfig {
    TestServerConfig::new()
}

impl TestServerConfig {
    /// Create default server configuration
    pub(crate) fn new() -> TestServerConfig {
        TestServerConfig {
            tp: HttpVer::Both,
            stream: StreamType::Tcp,
            client_timeout: 5000,
        }
    }

    /// Start http/1.1 server only
    pub fn h1(mut self) -> Self {
        self.tp = HttpVer::Http1;
        self
    }

    /// Start http/2 server only
    pub fn h2(mut self) -> Self {
        self.tp = HttpVer::Http2;
        self
    }

    /// Start rustls server
    pub fn rustls(mut self, config: rust_tls::ServerConfig) -> Self {
        self.stream = StreamType::Rustls(config);
        self
    }

    /// Set server client timeout in milliseconds for first request.
    pub fn client_timeout(mut self, val: u64) -> Self {
        self.client_timeout = val;
        self
    }
}

/// Get first available unused address
pub fn unused_addr() -> net::SocketAddr {
    let addr: net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let socket = TcpBuilder::new_v4().unwrap();
    socket.bind(&addr).unwrap();
    socket.reuse_address(true).unwrap();
    let tcp = socket.to_tcp_listener().unwrap();
    tcp.local_addr().unwrap()
}

/// Test server controller
pub struct TestServer {
    addr: net::SocketAddr,
    client: crate::web::client::Client,
    system: crate::fiber::System,
    ssl: bool,
    server: Server,
}

impl TestServer {
    /// Construct test server url
    pub fn addr(&self) -> net::SocketAddr {
        self.addr
    }

    /// Construct test server url
    pub fn url(&self, uri: &str) -> String {
        let scheme = if self.ssl { "https" } else { "http" };

        if uri.starts_with('/') {
            format!("{}://localhost:{}{}", scheme, self.addr.port(), uri)
        } else {
            format!("{}://localhost:{}/{}", scheme, self.addr.port(), uri)
        }
    }

    /// Create `GET` request
    pub fn get<S: AsRef<str>>(&self, path: S) -> ClientRequest {
        self.client.get(self.url(path.as_ref()).as_str())
    }

    /// Create `POST` request
    pub fn post<S: AsRef<str>>(&self, path: S) -> ClientRequest {
        self.client.post(self.url(path.as_ref()).as_str())
    }

    /// Create `HEAD` request
    pub fn head<S: AsRef<str>>(&self, path: S) -> ClientRequest {
        self.client.head(self.url(path.as_ref()).as_str())
    }

    /// Create `PUT` request
    pub fn put<S: AsRef<str>>(&self, path: S) -> ClientRequest {
        self.client.put(self.url(path.as_ref()).as_str())
    }

    /// Create `PATCH` request
    pub fn patch<S: AsRef<str>>(&self, path: S) -> ClientRequest {
        self.client.patch(self.url(path.as_ref()).as_str())
    }

    /// Create `DELETE` request
    pub fn delete<S: AsRef<str>>(&self, path: S) -> ClientRequest {
        self.client.delete(self.url(path.as_ref()).as_str())
    }

    /// Create `OPTIONS` request
    pub fn options<S: AsRef<str>>(&self, path: S) -> ClientRequest {
        self.client.options(self.url(path.as_ref()).as_str())
    }

    /// Connect to test http server
    pub fn request<S: AsRef<str>>(&self, method: Method, path: S) -> ClientRequest {
        self.client.request(method, path.as_ref())
    }

    pub async fn load_body<S>(
        &mut self,
        mut response: ClientResponse<S>,
    ) -> Result<Bytes, PayloadError>
    where
        S: Stream<Item = Result<Bytes, PayloadError>> + Unpin + 'static,
    {
        response.body().limit(10_485_760).await
    }

    /// Connect to websocket server at a given path
    pub async fn ws_at(
        &mut self,
        path: &str,
    ) -> Result<Framed<impl AsyncRead + AsyncWrite, websocket::Codec>, crate::web::client::error::WsClientError>
    {
        let url = self.url(path);
        let connect = self.client.ws(url).connect();
        connect.await.map(|(_, framed)| framed)
    }

    /// Connect to a websocket server
    pub async fn ws(
        &mut self,
    ) -> Result<Framed<impl AsyncRead + AsyncWrite, websocket::Codec>, crate::web::client::error::WsClientError>
    {
        self.ws_at("/").await
    }

    /// Gracefully stop http server
    pub async fn stop(self) {
        self.server.stop(true).await;
        self.system.stop();
        delay_for(time::Duration::from_millis(100)).await;
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.system.stop()
    }
}

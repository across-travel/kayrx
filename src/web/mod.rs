//! Web framework and services of kayrx for the Rust.
//! 
//! ```rust
//! use kayrx::web::{web, App, HttpServer, Responder};
//!
//! #[get("/{id}/{name}/index.html")]
//! async fn index(info: web::Path<(u32, String)>) -> impl Responder {
//!     format!("Hello {}! id:{}", info.1, info.0)
//! }
//!
//! #[kayrx::main]
//! async fn main() -> std::io::Result<()> {
//!     HttpServer::new(|| App::new().service(index))
//!         .bind("127.0.0.1:8080")?
//!         .run()
//!         .await
//! }
//! ```
//! 
//! ## Documentation & Resources
//!
//!
//! * [Website](https://kayrx.github.io/kayrx/)
//! * [Examples](https://github.com/kayrx/awesome/tree/master/examples)
//!
//! To main API pages:
//!
//! * [App](struct.App.html): This struct represents an kayrx-web
//!   application and is used to configure routes and other common
//!   settings.
//!
//! * [HttpServer](struct.HttpServer.html): This struct
//!   represents an HTTP server instance and is used to instantiate and
//!   configure servers.
//!
//! * [web](web/index.html): This module
//!   provides essential helper functions and types for application registration.
//!
//! * [HttpRequest](struct.HttpRequest.html) and
//!   [HttpResponse](struct.HttpResponse.html): These structs
//!   represent HTTP requests and responses and expose various methods
//!   for inspecting, creating and otherwise utilizing them.
//!
//! ## Features
//!
//! * Supported *HTTP/1.x* and *HTTP/2.0* protocols
//! * Streaming and pipelining
//! * Keep-alive and slow requests handling
//! * `WebSockets` server/client
//! * Transparent content compression/decompression (br, gzip, deflate)
//! * Configurable request routing
//! * Multipart streams
//! * SSL support with Rustls
//! * Middlewares (`Logger`,  `CORS`, `DefaultHeaders` etc.)
//! * Static assets
//! * Async Web Client.
//!
//! ## Package feature
//!
//! * `cookie` - enables http cookie support.
//! 

mod app;
mod app_service;
mod config;
mod data;
mod extract;
mod handler;
mod info;
mod request;
mod resource;
mod responder;
mod rmap;
mod route;
mod scope;
mod server;
mod service;
mod web;

pub mod client;
pub mod error;
pub mod file;
pub mod guard;
pub mod middleware;
pub mod multipart;
pub mod test;
pub mod types;

pub use self::app::App;
pub use self::config::ServiceConfig;
pub use self::data::Data;
pub use self::extract::FromRequest;
pub use self::request::HttpRequest;
pub use self::resource::Resource;
pub use self::responder::{Either, Responder};
pub use self::route::Route;
pub use self::scope::Scope;
pub use self::server::HttpServer;
pub use self::service::WebService;
pub use self::web::*;

pub use crate::http::Response as HttpResponse;
pub use crate::http::ResponseBuilder as HttpResponseBuilder;

pub mod dev {
    //! The `kayrx` prelude for library developers
    //!
    //! The purpose of this module is to alleviate imports of many common kayrx
    //! traits by adding a glob import to the top of kayrx heavy modules:

    pub use crate::http::body::{Body, BodySize, MessageBody, ResponseBody, SizedStream};
    pub use crate::http::encoding::Decoder as Decompress;
    pub use crate::http::ResponseBuilder as HttpResponseBuilder;
    pub use crate::http::{ Extensions, Payload, PayloadStream, RequestHead, ResponseHead};
    pub use crate::server::Server;
    pub use crate::service::{Service, Transform};
    pub use crate::router::{Path, ResourceDef, ResourcePath, Url};
    pub use super::config::{AppConfig, AppService};
    #[doc(hidden)]
    pub use super::handler::Factory;
    pub use super::info::ConnectionInfo;
    pub use super::rmap::ResourceMap;
    pub use super::service::{HttpServiceFactory, ServiceRequest, ServiceResponse, WebService};
    pub use super::types::form::UrlEncoded;
    pub use super::types::json::JsonBody;
    pub use super::types::readlines::Readlines;

    use crate::http::header::ContentEncoding;
    use crate::http::{Response, ResponseBuilder};
    

    pub(crate) fn insert_slash(mut patterns: Vec<String>) -> Vec<String> {
        for path in &mut patterns {
            if !path.is_empty() && !path.starts_with('/') {
                path.insert(0, '/');
            };
        }
        patterns
    }

    struct Enc(ContentEncoding);

    /// Helper trait that allows to set specific encoding for response.
    pub trait BodyEncoding {
        /// Get content encoding
        fn get_encoding(&self) -> Option<ContentEncoding>;

        /// Set content encoding
        fn encoding(&mut self, encoding: ContentEncoding) -> &mut Self;
    }

    impl BodyEncoding for ResponseBuilder {
        fn get_encoding(&self) -> Option<ContentEncoding> {
            if let Some(ref enc) = self.extensions().get::<Enc>() {
                Some(enc.0)
            } else {
                None
            }
        }

        fn encoding(&mut self, encoding: ContentEncoding) -> &mut Self {
            self.extensions_mut().insert(Enc(encoding));
            self
        }
    }

    impl<B> BodyEncoding for Response<B> {
        fn get_encoding(&self) -> Option<ContentEncoding> {
            if let Some(ref enc) = self.extensions().get::<Enc>() {
                Some(enc.0)
            } else {
                None
            }
        }

        fn encoding(&mut self, encoding: ContentEncoding) -> &mut Self {
            self.extensions_mut().insert(Enc(encoding));
            self
        }
    }
}

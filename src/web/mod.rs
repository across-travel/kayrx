mod app;
mod app_service;
mod config;
mod data;
pub mod error;
mod extract;
pub mod file;
pub mod guard;
mod handler;
mod info;
pub mod middleware;
pub mod multipart;
mod request;
mod resource;
mod responder;
mod rmap;
mod route;
mod scope;
mod server;
mod service;
pub mod test;
mod types;
pub mod web;
pub mod client;

pub use crate::http::Response as HttpResponse;
pub use crate::http::{body, Error, HttpMessage, ResponseError, Result};

pub use crate::web::app::App;
pub use crate::web::extract::FromRequest;
pub use crate::web::request::HttpRequest;
pub use crate::web::resource::Resource;
pub use crate::web::responder::{Either, Responder};
pub use crate::web::route::Route;
pub use crate::web::scope::Scope;
pub use crate::web::server::HttpServer;

pub mod dev {
    //! The `kayrx` prelude for library developers
    //!
    //! The purpose of this module is to alleviate imports of many common kayrx
    //! traits by adding a glob import to the top of kayrx heavy modules:
    //!
    //! ```
    //! # #![allow(unused_imports)]
    //! use kayrx::web::dev::*;
    //! ```

    pub use crate::http::body::{Body, BodySize, MessageBody, ResponseBody, SizedStream};
    pub use crate::http::encoding::Decoder as Decompress;
    pub use crate::http::ResponseBuilder as HttpResponseBuilder;
    pub use crate::http::{ Extensions, Payload, PayloadStream, RequestHead, ResponseHead};
    pub use crate::server::Server;
    pub use crate::service::{Service, Transform};
    use crate::http::{Response, ResponseBuilder};

    pub use crate::web::config::{AppConfig, AppService};
    pub use crate::router::{Path, ResourceDef, ResourcePath, Url};
    #[doc(hidden)]
    pub use crate::web::handler::Factory;
    pub use crate::web::info::ConnectionInfo;
    pub use crate::web::rmap::ResourceMap;
    pub use crate::web::service::{HttpServiceFactory, ServiceRequest, ServiceResponse, WebService};
    pub use crate::web::types::form::UrlEncoded;
    pub use crate::web::types::json::JsonBody;
    pub use crate::web::types::readlines::Readlines;

    use crate::http::header::ContentEncoding;
    

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


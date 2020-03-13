#![allow(clippy::borrow_interior_mutable_const, clippy::type_complexity)]

//! Static files support
use std::cell::RefCell;
use std::fmt::Write;
use std::fs::{DirEntry, File};
use std::future::Future;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::{cmp, io};

use crate::service::boxed::{self, BoxService, BoxServiceFactory};
use crate::service::{IntoServiceFactory, Service, ServiceFactory};
use crate::web::dev::{
    AppService, HttpServiceFactory, Payload, ResourceDef, ServiceRequest,
    ServiceResponse,
};
use crate::web::error::{BlockingError, Error, ErrorInternalServerError};
use crate::web::guard::Guard;
use crate::http::header::{self, DispositionType};
use crate::http::Method;
use crate::web::{web, FromRequest, HttpRequest};
use crate::http::Response as HttpResponse;
use bytes::Bytes;
use futures_util::future::{ok, ready, Either, FutureExt, LocalBoxFuture, Ready};
use futures_core::Stream;
use mime;
use mime_guess::from_ext;
use percent_encoding::{utf8_percent_encode, CONTROLS};
use v_htmlescape::escape as escape_html_entity;

mod error;
mod named;
mod range;

pub use self::error::{FilesError, UriSegmentError};
pub use self::named::NamedFile;
pub use self::range::HttpRange;

type HttpService = BoxService<ServiceRequest, ServiceResponse, Error>;
type HttpNewService = BoxServiceFactory<(), ServiceRequest, ServiceResponse, Error, ()>;

/// Return the MIME type associated with a filename extension (case-insensitive).
/// If `ext` is empty or no associated type for the extension was found, returns
/// the type `application/octet-stream`.
#[inline]
pub fn file_extension_to_mime(ext: &str) -> mime::Mime {
    from_ext(ext).first_or_octet_stream()
}

fn handle_error(err: BlockingError<io::Error>) -> Error {
    match err {
        BlockingError::Error(err) => err.into(),
        BlockingError::Canceled => ErrorInternalServerError("Unexpected error"),
    }
}
#[doc(hidden)]
/// A helper created from a `std::fs::File` which reads the file
/// chunk-by-chunk on a `ThreadPool`.
pub struct ChunkedReadFile {
    size: u64,
    offset: u64,
    file: Option<File>,
    fut:
        Option<LocalBoxFuture<'static, Result<(File, Bytes), BlockingError<io::Error>>>>,
    counter: u64,
}

impl Stream for ChunkedReadFile {
    type Item = Result<Bytes, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        if let Some(ref mut fut) = self.fut {
            return match Pin::new(fut).poll(cx) {
                Poll::Ready(Ok((file, bytes))) => {
                    self.fut.take();
                    self.file = Some(file);
                    self.offset += bytes.len() as u64;
                    self.counter += bytes.len() as u64;
                    Poll::Ready(Some(Ok(bytes)))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Some(Err(handle_error(e)))),
                Poll::Pending => Poll::Pending,
            };
        }

        let size = self.size;
        let offset = self.offset;
        let counter = self.counter;

        if size == counter {
            Poll::Ready(None)
        } else {
            let mut file = self.file.take().expect("Use after completion");
            self.fut = Some(
                web::block(move || {
                    let max_bytes: usize;
                    max_bytes = cmp::min(size.saturating_sub(counter), 65_536) as usize;
                    let mut buf = Vec::with_capacity(max_bytes);
                    file.seek(io::SeekFrom::Start(offset))?;
                    let nbytes =
                        file.by_ref().take(max_bytes as u64).read_to_end(&mut buf)?;
                    if nbytes == 0 {
                        return Err(io::ErrorKind::UnexpectedEof.into());
                    }
                    Ok((file, Bytes::from(buf)))
                })
                .boxed_local(),
            );
            self.poll_next(cx)
        }
    }
}

type DirectoryRenderer =
    dyn Fn(&Directory, &HttpRequest) -> Result<ServiceResponse, io::Error>;

/// A directory; responds with the generated directory listing.
#[derive(Debug)]
pub struct Directory {
    /// Base directory
    pub base: PathBuf,
    /// Path of subdirectory to generate listing for
    pub path: PathBuf,
}

impl Directory {
    /// Create a new directory
    pub fn new(base: PathBuf, path: PathBuf) -> Directory {
        Directory { base, path }
    }

    /// Is this entry visible from this directory?
    pub fn is_visible(&self, entry: &io::Result<DirEntry>) -> bool {
        if let Ok(ref entry) = *entry {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with('.') {
                    return false;
                }
            }
            if let Ok(ref md) = entry.metadata() {
                let ft = md.file_type();
                return ft.is_dir() || ft.is_file() || ft.is_symlink();
            }
        }
        false
    }
}

// show file url as relative to static path
macro_rules! encode_file_url {
    ($path:ident) => {
        utf8_percent_encode(&$path, CONTROLS)
    };
}

// " -- &quot;  & -- &amp;  ' -- &#x27;  < -- &lt;  > -- &gt;  / -- &#x2f;
macro_rules! encode_file_name {
    ($entry:ident) => {
        escape_html_entity(&$entry.file_name().to_string_lossy())
    };
}

fn directory_listing(
    dir: &Directory,
    req: &HttpRequest,
) -> Result<ServiceResponse, io::Error> {
    let index_of = format!("Index of {}", req.path());
    let mut body = String::new();
    let base = Path::new(req.path());

    for entry in dir.path.read_dir()? {
        if dir.is_visible(&entry) {
            let entry = entry.unwrap();
            let p = match entry.path().strip_prefix(&dir.path) {
                Ok(p) if cfg!(windows) => {
                    base.join(p).to_string_lossy().replace("\\", "/")
                }
                Ok(p) => base.join(p).to_string_lossy().into_owned(),
                Err(_) => continue,
            };

            // if file is a directory, add '/' to the end of the name
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    let _ = write!(
                        body,
                        "<li><a href=\"{}\">{}/</a></li>",
                        encode_file_url!(p),
                        encode_file_name!(entry),
                    );
                } else {
                    let _ = write!(
                        body,
                        "<li><a href=\"{}\">{}</a></li>",
                        encode_file_url!(p),
                        encode_file_name!(entry),
                    );
                }
            } else {
                continue;
            }
        }
    }

    let html = format!(
        "<html>\
         <head><title>{}</title></head>\
         <body><h1>{}</h1>\
         <ul>\
         {}\
         </ul></body>\n</html>",
        index_of, index_of, body
    );
    Ok(ServiceResponse::new(
        req.clone(),
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
    ))
}

type MimeOverride = dyn Fn(&mime::Name) -> DispositionType;

/// Static files handling
///
/// `Files` service must be registered with `App::service()` method.
///
/// ```rust
/// use kayrx::web::{App, file as fs};
///
/// fn main() {
///     let app = App::new()
///         .service(fs::Files::new("/static", "."));
/// }
/// ```
pub struct Files {
    path: String,
    directory: PathBuf,
    index: Option<String>,
    show_index: bool,
    redirect_to_slash: bool,
    default: Rc<RefCell<Option<Rc<HttpNewService>>>>,
    renderer: Rc<DirectoryRenderer>,
    mime_override: Option<Rc<MimeOverride>>,
    file_flags: named::Flags,
    guards: Option<Rc<Box<dyn Guard>>>,
}

impl Clone for Files {
    fn clone(&self) -> Self {
        Self {
            directory: self.directory.clone(),
            index: self.index.clone(),
            show_index: self.show_index,
            redirect_to_slash: self.redirect_to_slash,
            default: self.default.clone(),
            renderer: self.renderer.clone(),
            file_flags: self.file_flags,
            path: self.path.clone(),
            mime_override: self.mime_override.clone(),
            guards: self.guards.clone(),
        }
    }
}

impl Files {
    /// Create new `Files` instance for specified base directory.
    ///
    /// `File` uses `ThreadPool` for blocking filesystem operations.
    /// By default pool with 5x threads of available cpus is used.
    /// Pool size can be changed by setting ACTIX_CPU_POOL environment variable.
    pub fn new<T: Into<PathBuf>>(path: &str, dir: T) -> Files {
        let orig_dir = dir.into();
        let dir = match orig_dir.canonicalize() {
            Ok(canon_dir) => canon_dir,
            Err(_) => {
                log::error!("Specified path is not a directory: {:?}", orig_dir);
                PathBuf::new()
            }
        };

        Files {
            path: path.to_string(),
            directory: dir,
            index: None,
            show_index: false,
            redirect_to_slash: false,
            default: Rc::new(RefCell::new(None)),
            renderer: Rc::new(directory_listing),
            mime_override: None,
            file_flags: named::Flags::default(),
            guards: None,
        }
    }

    /// Show files listing for directories.
    ///
    /// By default show files listing is disabled.
    pub fn show_files_listing(mut self) -> Self {
        self.show_index = true;
        self
    }

    /// Redirects to a slash-ended path when browsing a directory.
    ///
    /// By default never redirect.
    pub fn redirect_to_slash_directory(mut self) -> Self {
        self.redirect_to_slash = true;
        self
    }

    /// Set custom directory renderer
    pub fn files_listing_renderer<F>(mut self, f: F) -> Self
    where
        for<'r, 's> F: Fn(&'r Directory, &'s HttpRequest) -> Result<ServiceResponse, io::Error>
            + 'static,
    {
        self.renderer = Rc::new(f);
        self
    }

    /// Specifies mime override callback
    pub fn mime_override<F>(mut self, f: F) -> Self
    where
        F: Fn(&mime::Name) -> DispositionType + 'static,
    {
        self.mime_override = Some(Rc::new(f));
        self
    }

    /// Set index file
    ///
    /// Shows specific index file for directory "/" instead of
    /// showing files listing.
    pub fn index_file<T: Into<String>>(mut self, index: T) -> Self {
        self.index = Some(index.into());
        self
    }

    #[inline]
    /// Specifies whether to use ETag or not.
    ///
    /// Default is true.
    pub fn use_etag(mut self, value: bool) -> Self {
        self.file_flags.set(named::Flags::ETAG, value);
        self
    }

    #[inline]
    /// Specifies whether to use Last-Modified or not.
    ///
    /// Default is true.
    pub fn use_last_modified(mut self, value: bool) -> Self {
        self.file_flags.set(named::Flags::LAST_MD, value);
        self
    }

    /// Specifies custom guards to use for directory listings and files.
    ///
    /// Default behaviour allows GET and HEAD.
    #[inline]
    pub fn use_guards<G: Guard + 'static>(mut self, guards: G) -> Self {
        self.guards = Some(Rc::new(Box::new(guards)));
        self
    }

    /// Disable `Content-Disposition` header.
    ///
    /// By default Content-Disposition` header is enabled.
    #[inline]
    pub fn disable_content_disposition(mut self) -> Self {
        self.file_flags.remove(named::Flags::CONTENT_DISPOSITION);
        self
    }

    /// Sets default handler which is used when no matched file could be found.
    pub fn default_handler<F, U>(mut self, f: F) -> Self
    where
        F: IntoServiceFactory<U>,
        U: ServiceFactory<
                Config = (),
                Request = ServiceRequest,
                Response = ServiceResponse,
                Error = Error,
            > + 'static,
    {
        // create and configure default resource
        self.default = Rc::new(RefCell::new(Some(Rc::new(boxed::factory(
            f.into_factory().map_init_err(|_| ()),
        )))));

        self
    }
}

impl HttpServiceFactory for Files {
    fn register(self, config: &mut AppService) {
        if self.default.borrow().is_none() {
            *self.default.borrow_mut() = Some(config.default_service());
        }
        let rdef = if config.is_root() {
            ResourceDef::root_prefix(&self.path)
        } else {
            ResourceDef::prefix(&self.path)
        };
        config.register_service(rdef, None, self, None)
    }
}

impl ServiceFactory for Files {
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type Config = ();
    type Service = FilesService;
    type InitError = ();
    type Future = LocalBoxFuture<'static, Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: ()) -> Self::Future {
        let mut srv = FilesService {
            directory: self.directory.clone(),
            index: self.index.clone(),
            show_index: self.show_index,
            redirect_to_slash: self.redirect_to_slash,
            default: None,
            renderer: self.renderer.clone(),
            mime_override: self.mime_override.clone(),
            file_flags: self.file_flags,
            guards: self.guards.clone(),
        };

        if let Some(ref default) = *self.default.borrow() {
            default
                .new_service(())
                .map(move |result| match result {
                    Ok(default) => {
                        srv.default = Some(default);
                        Ok(srv)
                    }
                    Err(_) => Err(()),
                })
                .boxed_local()
        } else {
            ok(srv).boxed_local()
        }
    }
}

pub struct FilesService {
    directory: PathBuf,
    index: Option<String>,
    show_index: bool,
    redirect_to_slash: bool,
    default: Option<HttpService>,
    renderer: Rc<DirectoryRenderer>,
    mime_override: Option<Rc<MimeOverride>>,
    file_flags: named::Flags,
    guards: Option<Rc<Box<dyn Guard>>>,
}

impl FilesService {
    fn handle_err(
        &mut self,
        e: io::Error,
        req: ServiceRequest,
    ) -> Either<
        Ready<Result<ServiceResponse, Error>>,
        LocalBoxFuture<'static, Result<ServiceResponse, Error>>,
    > {
        log::debug!("Files: Failed to handle {}: {}", req.path(), e);
        if let Some(ref mut default) = self.default {
            Either::Right(default.call(req))
        } else {
            Either::Left(ok(req.error_response(e)))
        }
    }
}

impl Service for FilesService {
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Either<
        Ready<Result<Self::Response, Self::Error>>,
        LocalBoxFuture<'static, Result<Self::Response, Self::Error>>,
    >;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let is_method_valid = if let Some(guard) = &self.guards {
            // execute user defined guards
            (**guard).check(req.head())
        } else {
            // default behaviour
            match *req.method() {
                Method::HEAD | Method::GET => true,
                _ => false,
            }
        };

        if !is_method_valid {
            return Either::Left(ok(req.into_response(
                HttpResponse::MethodNotAllowed()
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body("Request did not meet this resource's requirements."),
            )));
        }

        let real_path = match PathBufWrp::get_pathbuf(req.match_info().path()) {
            Ok(item) => item,
            Err(e) => return Either::Left(ok(req.error_response(e))),
        };

        // full filepath
        let path = match self.directory.join(&real_path.0).canonicalize() {
            Ok(path) => path,
            Err(e) => return self.handle_err(e, req),
        };

        if path.is_dir() {
            if let Some(ref redir_index) = self.index {
                if self.redirect_to_slash && !req.path().ends_with('/') {
                    let redirect_to = format!("{}/", req.path());
                    return Either::Left(ok(req.into_response(
                        HttpResponse::Found()
                            .header(header::LOCATION, redirect_to)
                            .body("")
                            .into_body(),
                    )));
                }

                let path = path.join(redir_index);

                match NamedFile::open(path) {
                    Ok(mut named_file) => {
                        if let Some(ref mime_override) = self.mime_override {
                            let new_disposition =
                                mime_override(&named_file.content_type.type_());
                            named_file.content_disposition.disposition = new_disposition;
                        }

                        named_file.flags = self.file_flags;
                        let (req, _) = req.into_parts();
                        Either::Left(ok(match named_file.into_response(&req) {
                            Ok(item) => ServiceResponse::new(req, item),
                            Err(e) => ServiceResponse::from_err(e, req),
                        }))
                    }
                    Err(e) => self.handle_err(e, req),
                }
            } else if self.show_index {
                let dir = Directory::new(self.directory.clone(), path);
                let (req, _) = req.into_parts();
                let x = (self.renderer)(&dir, &req);
                match x {
                    Ok(resp) => Either::Left(ok(resp)),
                    Err(e) => Either::Left(ok(ServiceResponse::from_err(e, req))),
                }
            } else {
                Either::Left(ok(ServiceResponse::from_err(
                    FilesError::IsDirectory,
                    req.into_parts().0,
                )))
            }
        } else {
            match NamedFile::open(path) {
                Ok(mut named_file) => {
                    if let Some(ref mime_override) = self.mime_override {
                        let new_disposition =
                            mime_override(&named_file.content_type.type_());
                        named_file.content_disposition.disposition = new_disposition;
                    }

                    named_file.flags = self.file_flags;
                    let (req, _) = req.into_parts();
                    match named_file.into_response(&req) {
                        Ok(item) => {
                            Either::Left(ok(ServiceResponse::new(req.clone(), item)))
                        }
                        Err(e) => Either::Left(ok(ServiceResponse::from_err(e, req))),
                    }
                }
                Err(e) => self.handle_err(e, req),
            }
        }
    }
}

#[derive(Debug)]
pub struct PathBufWrp(pub PathBuf);

impl PathBufWrp {
    pub fn get_pathbuf(path: &str) -> Result<Self, UriSegmentError> {
        let mut buf = PathBuf::new();
        for segment in path.split('/') {
            if segment == ".." {
                buf.pop();
            } else if segment.starts_with('.') {
                return Err(UriSegmentError::BadStart('.'));
            } else if segment.starts_with('*') {
                return Err(UriSegmentError::BadStart('*'));
            } else if segment.ends_with(':') {
                return Err(UriSegmentError::BadEnd(':'));
            } else if segment.ends_with('>') {
                return Err(UriSegmentError::BadEnd('>'));
            } else if segment.ends_with('<') {
                return Err(UriSegmentError::BadEnd('<'));
            } else if segment.is_empty() {
                continue;
            } else if cfg!(windows) && segment.contains('\\') {
                return Err(UriSegmentError::BadChar('\\'));
            } else {
                buf.push(segment)
            }
        }

        Ok(PathBufWrp(buf))
    }
}

impl FromRequest for PathBufWrp {
    type Error = UriSegmentError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(PathBufWrp::get_pathbuf(req.match_info().path()))
    }
}

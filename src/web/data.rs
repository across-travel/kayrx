use std::ops::Deref;
use std::sync::Arc;

use crate::http::error::{Error, ErrorInternalServerError};
use crate::http::Extensions;
use futures_util::future::{err, ok, Ready};

use crate::web::dev::Payload;
use crate::web::extract::FromRequest;
use crate::web::request::HttpRequest;

/// Application data factory
pub(crate) trait DataFactory {
    fn create(&self, extensions: &mut Extensions) -> bool;
}

/// Application data.
///
/// Application data is an arbitrary data attached to the app.
/// Application data is available to all routes and could be added
/// during application configuration process
/// with `App::data()` method.
///
/// Application data could be accessed by using `Data<T>`
/// extractor where `T` is data type.
///
/// **Note**: http server accepts an application factory rather than
/// an application instance. Http server constructs an application
/// instance for each thread, thus application data must be constructed
/// multiple times. If you want to share data between different
/// threads, a shareable object should be used, e.g. `Send + Sync`. Application
/// data does not need to be `Send` or `Sync`. Internally `Data` type
/// uses `Arc`. if your data implements `Send` + `Sync` traits you can
/// use `web::Data::new()` and avoid double `Arc`.
///
/// If route data is not set for a handler, using `Data<T>` extractor would
/// cause *Internal Server Error* response.
///
/// ```rust
/// use std::sync::Mutex;
/// use kayrx::web::{web, App, HttpResponse, Responder};
///
/// struct MyData {
///     counter: usize,
/// }
///
/// /// Use `Data<T>` extractor to access data in handler.
/// async fn index(data: web::Data<Mutex<MyData>>) -> impl Responder {
///     let mut data = data.lock().unwrap();
///     data.counter += 1;
///     HttpResponse::Ok()
/// }
///
/// fn main() {
///     let data = web::Data::new(Mutex::new(MyData{ counter: 0 }));
///
///     let app = App::new()
///         // Store `MyData` in application storage.
///         .app_data(data.clone())
///         .service(
///             web::resource("/index.html").route(
///                 web::get().to(index)));
/// }
/// ```
#[derive(Debug)]
pub struct Data<T>(Arc<T>);

impl<T> Data<T> {
    /// Create new `Data` instance.
    ///
    /// Internally `Data` type uses `Arc`. if your data implements
    /// `Send` + `Sync` traits you can use `web::Data::new()` and
    /// avoid double `Arc`.
    pub fn new(state: T) -> Data<T> {
        Data(Arc::new(state))
    }

    /// Get reference to inner app data.
    pub fn get_ref(&self) -> &T {
        self.0.as_ref()
    }

    /// Convert to the internal Arc<T>
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

impl<T> Deref for Data<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Arc<T> {
        &self.0
    }
}

impl<T> Clone for Data<T> {
    fn clone(&self) -> Data<T> {
        Data(self.0.clone())
    }
}

impl<T: 'static> FromRequest for Data<T> {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(st) = req.app_data::<Data<T>>() {
            ok(st.clone())
        } else {
            log::debug!(
                "Failed to construct App-level Data extractor. \
                 Request path: {:?}",
                req.path()
            );
            err(ErrorInternalServerError(
                "App data is not configured, to configure use App::data()",
            ))
        }
    }
}

impl<T: 'static> DataFactory for Data<T> {
    fn create(&self, extensions: &mut Extensions) -> bool {
        if !extensions.contains::<Data<T>>() {
            extensions.insert(Data(self.0.clone()));
            true
        } else {
            false
        }
    }
}

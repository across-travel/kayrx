use std::cell::RefCell;
use std::fmt;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use futures_util::future::{ok, Either, LocalBoxFuture, Ready};
use futures_core::Future;

use crate::http::{Extensions, Response};
use crate::router::{ResourceDef, ResourceInfo, Router};
use crate::service::boxed::{self, BoxService, BoxServiceFactory};
use crate::service::{ apply, apply_fn_factory, IntoServiceFactory, Service, ServiceFactory, Transform};
use crate::web::config::ServiceConfig;
use crate::web::data::Data;
use crate::web::dev::{AppService, HttpServiceFactory};
use crate::web::error::Error;
use crate::web::guard::Guard;
use crate::web::resource::Resource;
use crate::web::rmap::ResourceMap;
use crate::web::route::Route;
use crate::web::service::{ AppServiceFactory, ServiceFactoryWrapper, ServiceRequest, ServiceResponse};

type Guards = Vec<Box<dyn Guard>>;
type HttpService = BoxService<ServiceRequest, ServiceResponse, Error>;
type HttpNewService = BoxServiceFactory<(), ServiceRequest, ServiceResponse, Error, ()>;
type BoxedResponse = LocalBoxFuture<'static, Result<ServiceResponse, Error>>;

/// Resources scope.
///
/// Scope is a set of resources with common root path.
/// Scopes collect multiple paths under a common path prefix.
/// Scope path can contain variable path segments as resources.
/// Scope prefix is always complete path segment, i.e `/app` would
/// be converted to a `/app/` and it would not match `/app` path.
///
/// You can get variable path segments from `HttpRequest::match_info()`.
/// `Path` extractor also is able to extract scope level variable segments.
///
/// ```rust
/// use kayrx::web::{web, App, HttpResponse};
///
/// fn main() {
///     let app = App::new().service(
///         web::scope("/{project_id}/")
///             .service(web::resource("/path1").to(|| async { HttpResponse::Ok() }))
///             .service(web::resource("/path2").route(web::get().to(|| HttpResponse::Ok())))
///             .service(web::resource("/path3").route(web::head().to(|| HttpResponse::MethodNotAllowed())))
///     );
/// }
/// ```
///
/// In the above example three routes get registered:
///  * /{project_id}/path1 - reponds to all http method
///  * /{project_id}/path2 - `GET` requests
///  * /{project_id}/path3 - `HEAD` requests
///
pub struct Scope<T = ScopeEndpoint> {
    endpoint: T,
    rdef: String,
    data: Option<Extensions>,
    services: Vec<Box<dyn AppServiceFactory>>,
    guards: Vec<Box<dyn Guard>>,
    default: Rc<RefCell<Option<Rc<HttpNewService>>>>,
    external: Vec<ResourceDef>,
    factory_ref: Rc<RefCell<Option<ScopeFactory>>>,
}

impl Scope {
    /// Create a new scope
    pub fn new(path: &str) -> Scope {
        let fref = Rc::new(RefCell::new(None));
        Scope {
            endpoint: ScopeEndpoint::new(fref.clone()),
            rdef: path.to_string(),
            data: None,
            guards: Vec::new(),
            services: Vec::new(),
            default: Rc::new(RefCell::new(None)),
            external: Vec::new(),
            factory_ref: fref,
        }
    }
}

impl<T> Scope<T>
where
    T: ServiceFactory<
        Config = (),
        Request = ServiceRequest,
        Response = ServiceResponse,
        Error = Error,
        InitError = (),
    >,
{
    /// Add match guard to a scope.
    ///
    /// ```rust
    /// use kayrx::web::{web, guard, App, HttpRequest, HttpResponse};
    ///
    /// async fn index(data: web::Path<(String, String)>) -> &'static str {
    ///     "Welcome!"
    /// }
    ///
    /// fn main() {
    ///     let app = App::new().service(
    ///         web::scope("/app")
    ///             .guard(guard::Header("content-type", "text/plain"))
    ///             .route("/test1", web::get().to(index))
    ///             .route("/test2", web::post().to(|r: HttpRequest| {
    ///                 HttpResponse::MethodNotAllowed()
    ///             }))
    ///     );
    /// }
    /// ```
    pub fn guard<G: Guard + 'static>(mut self, guard: G) -> Self {
        self.guards.push(Box::new(guard));
        self
    }

    /// Set or override application data. Application data could be accessed
    /// by using `Data<T>` extractor where `T` is data type.
    ///
    /// ```rust
    /// use std::cell::Cell;
    /// use kayrx::web::{web, App, HttpResponse, Responder};
    ///
    /// struct MyData {
    ///     counter: Cell<usize>,
    /// }
    ///
    /// async fn index(data: web::Data<MyData>) -> impl Responder {
    ///     data.counter.set(data.counter.get() + 1);
    ///     HttpResponse::Ok()
    /// }
    ///
    /// fn main() {
    ///     let app = App::new().service(
    ///         web::scope("/app")
    ///             .data(MyData{ counter: Cell::new(0) })
    ///             .service(
    ///                 web::resource("/index.html").route(
    ///                     web::get().to(index)))
    ///     );
    /// }
    /// ```
    pub fn data<U: 'static>(self, data: U) -> Self {
        self.app_data(Data::new(data))
    }

    /// Set or override application data.
    ///
    /// This method overrides data stored with [`App::app_data()`](#method.app_data)
    pub fn app_data<U: 'static>(mut self, data: U) -> Self {
        if self.data.is_none() {
            self.data = Some(Extensions::new());
        }
        self.data.as_mut().unwrap().insert(data);
        self
    }

    /// Run external configuration as part of the scope building
    /// process
    ///
    /// This function is useful for moving parts of configuration to a
    /// different module or even library. For example,
    /// some of the resource's configuration could be moved to different module.
    ///
    /// ```rust
    /// use kayrx::web::{web, middleware, App, HttpResponse};
    ///
    /// // this function could be located in different module
    /// fn config(cfg: &mut web::ServiceConfig) {
    ///     cfg.service(web::resource("/test")
    ///         .route(web::get().to(|| HttpResponse::Ok()))
    ///         .route(web::head().to(|| HttpResponse::MethodNotAllowed()))
    ///     );
    /// }
    ///
    /// fn main() {
    ///     let app = App::new()
    ///         .wrap(middleware::Logger::default())
    ///         .service(
    ///             web::scope("/api")
    ///                 .configure(config)
    ///         )
    ///         .route("/index.html", web::get().to(|| HttpResponse::Ok()));
    /// }
    /// ```
    pub fn configure<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut ServiceConfig),
    {
        let mut cfg = ServiceConfig::new();
        f(&mut cfg);
        self.services.extend(cfg.services);
        self.external.extend(cfg.external);

        if !cfg.data.is_empty() {
            let mut data = self.data.unwrap_or_else(Extensions::new);

            for value in cfg.data.iter() {
                value.create(&mut data);
            }

            self.data = Some(data);
        }
        self
    }

    /// Register http service.
    ///
    /// This is similar to `App's` service registration.
    ///
    /// Actix web provides several services implementations:
    ///
    /// * *Resource* is an entry in resource table which corresponds to requested URL.
    /// * *Scope* is a set of resources with common root path.
    /// * "StaticFiles" is a service for static files support
    ///
    /// ```rust
    /// use kayrx::web::{web, App, HttpRequest};
    ///
    /// struct AppState;
    ///
    /// async fn index(req: HttpRequest) -> &'static str {
    ///     "Welcome!"
    /// }
    ///
    /// fn main() {
    ///     let app = App::new().service(
    ///         web::scope("/app").service(
    ///             web::scope("/v1")
    ///                 .service(web::resource("/test1").to(index)))
    ///     );
    /// }
    /// ```
    pub fn service<F>(mut self, factory: F) -> Self
    where
        F: HttpServiceFactory + 'static,
    {
        self.services
            .push(Box::new(ServiceFactoryWrapper::new(factory)));
        self
    }

    /// Configure route for a specific path.
    ///
    /// This is a simplified version of the `Scope::service()` method.
    /// This method can be called multiple times, in that case
    /// multiple resources with one route would be registered for same resource path.
    ///
    /// ```rust
    /// use kayrx::web::{web, App, HttpResponse};
    ///
    /// async fn index(data: web::Path<(String, String)>) -> &'static str {
    ///     "Welcome!"
    /// }
    ///
    /// fn main() {
    ///     let app = App::new().service(
    ///         web::scope("/app")
    ///             .route("/test1", web::get().to(index))
    ///             .route("/test2", web::post().to(|| HttpResponse::MethodNotAllowed()))
    ///     );
    /// }
    /// ```
    pub fn route(self, path: &str, mut route: Route) -> Self {
        self.service(
            Resource::new(path)
                .add_guards(route.take_guards())
                .route(route),
        )
    }

    /// Default service to be used if no matching route could be found.
    ///
    /// If default resource is not registered, app's default resource is being used.
    pub fn default_service<F, U>(mut self, f: F) -> Self
    where
        F: IntoServiceFactory<U>,
        U: ServiceFactory<
                Config = (),
                Request = ServiceRequest,
                Response = ServiceResponse,
                Error = Error,
            > + 'static,
        U::InitError: fmt::Debug,
    {
        // create and configure default resource
        self.default = Rc::new(RefCell::new(Some(Rc::new(boxed::factory(
            f.into_factory().map_init_err(|e| {
                log::error!("Can not construct default service: {:?}", e)
            }),
        )))));

        self
    }

    /// Registers middleware, in the form of a middleware component (type),
    /// that runs during inbound processing in the request
    /// lifecycle (request -> response), modifying request as
    /// necessary, across all requests managed by the *Scope*.  Scope-level
    /// middleware is more limited in what it can modify, relative to Route or
    /// Application level middleware, in that Scope-level middleware can not modify
    /// ServiceResponse.
    ///
    /// Use middleware when you need to read or modify *every* request in some way.
    pub fn wrap<M>(
        self,
        mw: M,
    ) -> Scope<
        impl ServiceFactory<
            Config = (),
            Request = ServiceRequest,
            Response = ServiceResponse,
            Error = Error,
            InitError = (),
        >,
    >
    where
        M: Transform<
            T::Service,
            Request = ServiceRequest,
            Response = ServiceResponse,
            Error = Error,
            InitError = (),
        >,
    {
        Scope {
            endpoint: apply(mw, self.endpoint),
            rdef: self.rdef,
            data: self.data,
            guards: self.guards,
            services: self.services,
            default: self.default,
            external: self.external,
            factory_ref: self.factory_ref,
        }
    }

    /// Registers middleware, in the form of a closure, that runs during inbound
    /// processing in the request lifecycle (request -> response), modifying
    /// request as necessary, across all requests managed by the *Scope*.
    /// Scope-level middleware is more limited in what it can modify, relative
    /// to Route or Application level middleware, in that Scope-level middleware
    /// can not modify ServiceResponse.
    ///
    /// ```rust
    /// use kayrx::service::Service;
    /// use kayrx::web::{web, App};
    /// use kayrx::http::{header::CONTENT_TYPE, HeaderValue};
    ///
    /// async fn index() -> &'static str {
    ///     "Welcome!"
    /// }
    ///
    /// fn main() {
    ///     let app = App::new().service(
    ///         web::scope("/app")
    ///             .wrap_fn(|req, srv| {
    ///                 let fut = srv.call(req);
    ///                 async {
    ///                     let mut res = fut.await?;
    ///                     res.headers_mut().insert(
    ///                        CONTENT_TYPE, HeaderValue::from_static("text/plain"),
    ///                     );
    ///                     Ok(res)
    ///                 }
    ///             })
    ///             .route("/index.html", web::get().to(index)));
    /// }
    /// ```
    pub fn wrap_fn<F, R>(
        self,
        mw: F,
    ) -> Scope<
        impl ServiceFactory<
            Config = (),
            Request = ServiceRequest,
            Response = ServiceResponse,
            Error = Error,
            InitError = (),
        >,
    >
    where
        F: FnMut(ServiceRequest, &mut T::Service) -> R + Clone,
        R: Future<Output = Result<ServiceResponse, Error>>,
    {
        Scope {
            endpoint: apply_fn_factory(self.endpoint, mw),
            rdef: self.rdef,
            data: self.data,
            guards: self.guards,
            services: self.services,
            default: self.default,
            external: self.external,
            factory_ref: self.factory_ref,
        }
    }
}

impl<T> HttpServiceFactory for Scope<T>
where
    T: ServiceFactory<
            Config = (),
            Request = ServiceRequest,
            Response = ServiceResponse,
            Error = Error,
            InitError = (),
        > + 'static,
{
    fn register(mut self, config: &mut AppService) {
        // update default resource if needed
        if self.default.borrow().is_none() {
            *self.default.borrow_mut() = Some(config.default_service());
        }

        // register nested services
        let mut cfg = config.clone_config();
        self.services
            .into_iter()
            .for_each(|mut srv| srv.register(&mut cfg));

        let mut rmap = ResourceMap::new(ResourceDef::root_prefix(&self.rdef));

        // external resources
        for mut rdef in std::mem::replace(&mut self.external, Vec::new()) {
            rmap.add(&mut rdef, None);
        }

        // custom app data storage
        if let Some(ref mut ext) = self.data {
            config.set_service_data(ext);
        }

        // complete scope pipeline creation
        *self.factory_ref.borrow_mut() = Some(ScopeFactory {
            data: self.data.take().map(Rc::new),
            default: self.default.clone(),
            services: Rc::new(
                cfg.into_services()
                    .1
                    .into_iter()
                    .map(|(mut rdef, srv, guards, nested)| {
                        rmap.add(&mut rdef, nested);
                        (rdef, srv, RefCell::new(guards))
                    })
                    .collect(),
            ),
        });

        // get guards
        let guards = if self.guards.is_empty() {
            None
        } else {
            Some(self.guards)
        };

        // register final service
        config.register_service(
            ResourceDef::root_prefix(&self.rdef),
            guards,
            self.endpoint,
            Some(Rc::new(rmap)),
        )
    }
}

pub struct ScopeFactory {
    data: Option<Rc<Extensions>>,
    services: Rc<Vec<(ResourceDef, HttpNewService, RefCell<Option<Guards>>)>>,
    default: Rc<RefCell<Option<Rc<HttpNewService>>>>,
}

impl ServiceFactory for ScopeFactory {
    type Config = ();
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Service = ScopeService;
    type Future = ScopeFactoryResponse;

    fn new_service(&self, _: ()) -> Self::Future {
        let default_fut = if let Some(ref default) = *self.default.borrow() {
            Some(default.new_service(()))
        } else {
            None
        };

        ScopeFactoryResponse {
            fut: self
                .services
                .iter()
                .map(|(path, service, guards)| {
                    CreateScopeServiceItem::Future(
                        Some(path.clone()),
                        guards.borrow_mut().take(),
                        service.new_service(()),
                    )
                })
                .collect(),
            default: None,
            data: self.data.clone(),
            default_fut,
        }
    }
}

/// Create scope service
#[doc(hidden)]
#[pin_project::pin_project]
pub struct ScopeFactoryResponse {
    fut: Vec<CreateScopeServiceItem>,
    data: Option<Rc<Extensions>>,
    default: Option<HttpService>,
    default_fut: Option<LocalBoxFuture<'static, Result<HttpService, ()>>>,
}

type HttpServiceFut = LocalBoxFuture<'static, Result<HttpService, ()>>;

enum CreateScopeServiceItem {
    Future(Option<ResourceDef>, Option<Guards>, HttpServiceFut),
    Service(ResourceDef, Option<Guards>, HttpService),
}

impl Future for ScopeFactoryResponse {
    type Output = Result<ScopeService, ()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut done = true;

        if let Some(ref mut fut) = self.default_fut {
            match Pin::new(fut).poll(cx)? {
                Poll::Ready(default) => self.default = Some(default),
                Poll::Pending => done = false,
            }
        }

        // poll http services
        for item in &mut self.fut {
            let res = match item {
                CreateScopeServiceItem::Future(
                    ref mut path,
                    ref mut guards,
                    ref mut fut,
                ) => match Pin::new(fut).poll(cx)? {
                    Poll::Ready(service) => {
                        Some((path.take().unwrap(), guards.take(), service))
                    }
                    Poll::Pending => {
                        done = false;
                        None
                    }
                },
                CreateScopeServiceItem::Service(_, _, _) => continue,
            };

            if let Some((path, guards, service)) = res {
                *item = CreateScopeServiceItem::Service(path, guards, service);
            }
        }

        if done {
            let router = self
                .fut
                .drain(..)
                .fold(Router::build(), |mut router, item| {
                    match item {
                        CreateScopeServiceItem::Service(path, guards, service) => {
                            router.rdef(path, service).2 = guards;
                        }
                        CreateScopeServiceItem::Future(_, _, _) => unreachable!(),
                    }
                    router
                });
            Poll::Ready(Ok(ScopeService {
                data: self.data.clone(),
                router: router.finish(),
                default: self.default.take(),
                _ready: None,
            }))
        } else {
            Poll::Pending
        }
    }
}

pub struct ScopeService {
    data: Option<Rc<Extensions>>,
    router: Router<HttpService, Vec<Box<dyn Guard>>>,
    default: Option<HttpService>,
    _ready: Option<(ServiceRequest, ResourceInfo)>,
}

impl Service for ScopeService {
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Either<BoxedResponse, Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let res = self.router.recognize_mut_checked(&mut req, |req, guards| {
            if let Some(ref guards) = guards {
                for f in guards {
                    if !f.check(req.head()) {
                        return false;
                    }
                }
            }
            true
        });

        if let Some((srv, _info)) = res {
            if let Some(ref data) = self.data {
                req.set_data_container(data.clone());
            }
            Either::Left(srv.call(req))
        } else if let Some(ref mut default) = self.default {
            Either::Left(default.call(req))
        } else {
            let req = req.into_parts().0;
            Either::Right(ok(ServiceResponse::new(req, Response::NotFound().finish())))
        }
    }
}

#[doc(hidden)]
pub struct ScopeEndpoint {
    factory: Rc<RefCell<Option<ScopeFactory>>>,
}

impl ScopeEndpoint {
    fn new(factory: Rc<RefCell<Option<ScopeFactory>>>) -> Self {
        ScopeEndpoint { factory }
    }
}

impl ServiceFactory for ScopeEndpoint {
    type Config = ();
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Service = ScopeService;
    type Future = ScopeFactoryResponse;

    fn new_service(&self, _: ()) -> Self::Future {
        self.factory.borrow_mut().as_mut().unwrap().new_service(())
    }
}

//! Tcp connector service
//!
//! ## Package feature
//!
//! * `rustls` - enables ssl support via `rustls` crate


mod connect;
mod connector;
mod error;
mod resolve;
mod service;
pub mod ssl;

mod uri;

use crate::{krse::net::TcpStream,fiber:: Arbiter};
use crate::service::{pipeline, pipeline_factory, Service, ServiceFactory};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::system_conf::read_system_conf;
use trust_dns_resolver::AsyncResolver;

pub mod resolver {
    pub use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
    pub use trust_dns_resolver::system_conf::read_system_conf;
    pub use trust_dns_resolver::{error::ResolveError, AsyncResolver};
}

pub use self::connect::{Address, Connect, Connection};
pub use self::connector::{TcpConnector, TcpConnectorFactory};
pub use self::error::ConnectError;
pub use self::resolve::{Resolver, ResolverFactory};
pub use self::service::{ConnectService, ConnectServiceFactory, TcpConnectService};

pub fn start_resolver(cfg: ResolverConfig, opts: ResolverOpts) -> AsyncResolver {
    let (resolver, bg) = AsyncResolver::new(cfg, opts);
    crate::fiber::spawn(bg);
    resolver
}

struct DefaultResolver(AsyncResolver);

pub(crate) fn get_default_resolver() -> AsyncResolver {
    if Arbiter::contains_item::<DefaultResolver>() {
        Arbiter::get_item(|item: &DefaultResolver| item.0.clone())
    } else {
        let (cfg, opts) = match read_system_conf() {
            Ok((cfg, opts)) => (cfg, opts),
            Err(e) => {
                log::error!("TRust-DNS can not load system config: {}", e);
                (ResolverConfig::default(), ResolverOpts::default())
            }
        };

        let (resolver, bg) = AsyncResolver::new(cfg, opts);
        crate::fiber::spawn(bg);

        Arbiter::set_item(DefaultResolver(resolver.clone()));
        resolver
    }
}

pub fn start_default_resolver() -> AsyncResolver {
    get_default_resolver()
}

/// Create tcp connector service
pub fn new_connector<T: Address>(
    resolver: AsyncResolver,
) -> impl Service<Request = Connect<T>, Response = Connection<T, TcpStream>, Error = ConnectError>
       + Clone {
    pipeline(Resolver::new(resolver)).and_then(TcpConnector::new())
}

/// Create tcp connector service
pub fn new_connector_factory<T: Address>(
    resolver: AsyncResolver,
) -> impl ServiceFactory<
    Config = (),
    Request = Connect<T>,
    Response = Connection<T, TcpStream>,
    Error = ConnectError,
    InitError = (),
> + Clone {
    pipeline_factory(ResolverFactory::new(resolver)).and_then(TcpConnectorFactory::new())
}

/// Create connector service with default parameters
pub fn default_connector<T: Address>(
) -> impl Service<Request = Connect<T>, Response = Connection<T, TcpStream>, Error = ConnectError>
       + Clone {
    pipeline(Resolver::default()).and_then(TcpConnector::new())
}

/// Create connector service factory with default parameters
pub fn default_connector_factory<T: Address>() -> impl ServiceFactory<
    Config = (),
    Request = Connect<T>,
    Response = Connection<T, TcpStream>,
    Error = ConnectError,
    InitError = (),
> + Clone {
    pipeline_factory(ResolverFactory::default()).and_then(TcpConnectorFactory::new())
}

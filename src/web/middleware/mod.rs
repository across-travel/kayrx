//! Middlewares

mod compress;
mod condition;
mod cors;
mod defaultheaders;
pub mod errhandlers;
mod logger;
mod normalize;

pub use self::cors::Cors;
pub use self::compress::Compress;
pub use self::condition::Condition;
pub use self::defaultheaders::DefaultHeaders;
pub use self::logger::Logger;
pub use self::normalize::NormalizePath;

pub mod dev {
    pub use super::logger::{Format, FormatDisplay};
    pub use super::cors::*;
}

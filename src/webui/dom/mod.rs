//! virtual-dom-rs provides a virtual dom implementation as well as an `html!` macro
//! that you can use to generate a virtual dom.
//!
//! The virtual dom works on both the client and server. On the client we'll render
//! to an `HtmlElement`, and on the server we render to a `String`.

pub mod node;
pub mod validation;
mod diff;
mod dom_updater;
mod patch;

pub use self::dom_updater::DomUpdater;
pub use self::node::{View, IterableNodes};

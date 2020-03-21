//! Vdom provides virtual dom implementation withn `html!` macro
//!
//! The virtual dom works on both the client and server. On the client we'll render
//! to an `HtmlElement`, and on the server we render to a `String`.

mod diff;
mod dom_updater;
mod node;
mod patch;
mod validation;

pub use self::diff::diff;
pub use self::dom_updater::DomUpdater;
pub use self::node::{VNode, VElement, VText, CreatedNode, View, DynClosure, Events, IterableNodes};
pub use self::patch::{Patch, patch};



//! Kept in it's own file to more easily import into the Percy book.

use crate::webui::vdom::diff::diff;
use crate::webui::vdom::patch::Patch;
use crate::webui::vdom::node::VNode;

/// Test that we generate the right Vec<Patch> for some start and end virtual dom.
pub struct DiffTestCase<'a> {
    // ex: "Patching root level nodes works"
    pub description: &'static str,
    // ex: html! { <div> </div> }
    pub old: VNode,
    // ex: html! { <strong> </strong> }
    pub new: VNode,
    // ex: vec![Patch::Replace(0, &html! { <strong></strong> })],
    pub expected: Vec<Patch<'a>>,
}

impl<'a> DiffTestCase<'a> {
    pub fn test(&self) {
        // ex: vec![Patch::Replace(0, &html! { <strong></strong> })],
        let patches = diff(&self.old, &self.new);

        assert_eq!(patches, self.expected, "{}", self.description);
    }
}

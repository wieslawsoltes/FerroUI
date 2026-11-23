//! Template parts the XAML compiler resolved (#1039, a prototype).
//!
//! Where the compiler sees both a control and its template, the build function of the template
//! hands over the named parts in the order the control declares them, so the control reads a
//! part by its position instead of hashing its name. Everything else keeps the name lookup: a
//! template loaded at run time, one built in code, and a part this set does not hold.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::{FerroObject, Ref};

/// The parts of one built template, in the order of `names`.
pub struct CompiledTemplateParts {
    names: &'static [&'static str],
    parts: Box<[Option<Ref<FerroObject>>]>,
}

impl CompiledTemplateParts {
    /// The part at `index`, when this set declares `name` there and the template has it.
    pub(super) fn slot(&self, index: usize, name: &str) -> Option<&Ref<FerroObject>> {
        if *self.names.get(index)? != name {
            return None;
        }
        self.parts.get(index)?.as_ref()
    }
}

ferroui_base::ferro_thread_local! {
    static PUBLISHED: RefCell<Option<Rc<CompiledTemplateParts>>> = const { RefCell::new(None) };
}

/// Called by the generated build function of a template, as its last statement.
pub fn publish_compiled_template_parts(names: &'static [&'static str], parts: &[Option<Ref<FerroObject>>]) {
    let set = CompiledTemplateParts { names, parts: parts.into() };
    PUBLISHED.with(|published| *published.borrow_mut() = Some(Rc::new(set)));
}

/// Takes what the last build function published. Taken before a build too, so that a template
/// built outside of a control cannot hand its parts to the next one.
pub(super) fn take_compiled_template_parts() -> Option<Rc<CompiledTemplateParts>> {
    PUBLISHED.with(|published| published.borrow_mut().take())
}

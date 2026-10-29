//! Declaration provenance for development state transfer across template builds.
//! The registry holds object identities weakly; text snapshots contain no UI state.
use ferroui_base::{BoxedValue, FerroObject, WeakRef};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone)]
pub struct Declaration {
    pub uri: Option<String>,
    pub line: i32,
    pub column: i32,
    /// Exact parser input for interpreted markup, including any development
    /// normalization. Compiled declarations use the session's build baseline.
    pub source: Option<Rc<str>>,
}
ferroui_base::ferro_thread_local! {
    static DECLARATIONS: RefCell<HashMap<usize, (WeakRef<FerroObject>, Declaration)>> = RefCell::new(HashMap::new());
}
pub(crate) fn record(value: &BoxedValue, declaration: &Declaration) {
    let Some(object) = ferroui_base::data::core::ValueTypes::as_object(&**value) else {
        return;
    };
    DECLARATIONS.with(|entries| {
        let mut entries = entries.borrow_mut();
        entries.retain(|_, (object, _)| object.upgrade().is_some());
        entries.insert(
            &*object as *const FerroObject as usize,
            (object.downgrade(), declaration.clone()),
        );
    });
}
pub fn declaration(object: &FerroObject) -> Option<Declaration> {
    DECLARATIONS.with(|entries| {
        entries
            .borrow()
            .get(&(object as *const FerroObject as usize))
            .filter(|(weak, _)| weak.upgrade().is_some())
            .map(|(_, declaration)| declaration.clone())
    })
}

//! Weak development provenance for resource roots built from linked documents.
use crate::{FerroObject, Ref, WeakRef};
use std::cell::RefCell;
crate::ferro_thread_local! {
    static SOURCES: RefCell<Vec<(WeakRef<FerroObject>, String)>> = const { RefCell::new(Vec::new()) };
}
/// Records the source after successful construction without retaining its graph.
pub fn register(root: &Ref<FerroObject>, uri: String) {
    SOURCES.with(|sources| {
        let mut sources = sources.borrow_mut();
        sources.retain(|(weak, _)| weak.upgrade().is_some_and(|object| !object.ptr_eq(root)));
        sources.push((root.downgrade(), uri));
    });
}
pub(crate) fn source(root: &FerroObject) -> Option<String> {
    SOURCES.with(|sources| {
        let mut sources = sources.borrow_mut();
        sources.retain(|(weak, _)| weak.upgrade().is_some());
        sources.iter().find_map(|(weak, uri)| {
            weak.upgrade()
                .filter(|object| std::ptr::eq(&**object, root))
                .map(|_| uri.clone())
        })
    })
}

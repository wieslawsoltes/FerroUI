//! Development identities do not become public names or retain controls.
use ferroui_base::metadata::MarkupValue;
use ferroui_base::{FerroObject, WeakRef};
use std::{cell::RefCell, collections::HashMap};
ferroui_base::ferro_thread_local! {
    static KEYS: RefCell<HashMap<usize, (WeakRef<FerroObject>, String)>> = RefCell::new(HashMap::new());
}
pub(crate) fn record(value: &MarkupValue, key: &str) {
    let Some(object) = value
        .as_ref()
        .and_then(|value| ferroui_base::data::core::ValueTypes::as_object(&**value))
    else {
        return;
    };
    KEYS.with(|entries| {
        let mut entries = entries.borrow_mut();
        entries.retain(|_, (weak, _)| weak.upgrade().is_some());
        entries.insert(
            &*object as *const FerroObject as usize,
            (object.downgrade(), key.to_owned()),
        );
    });
}
pub fn key(object: &FerroObject) -> Option<String> {
    KEYS.with(|entries| {
        entries
            .borrow()
            .get(&(object as *const FerroObject as usize))
            .filter(|(weak, _)| weak.upgrade().is_some())
            .map(|(_, key)| key.clone())
    })
}

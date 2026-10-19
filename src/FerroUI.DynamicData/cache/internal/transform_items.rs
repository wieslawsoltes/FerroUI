//! The `Transform` that the merging and subscribing operators apply inside (`Cache/Internal/
//! Transform.cs`, the form without `transformOnRefresh` and without an exception callback): each
//! item becomes the value of `factory`, an update a new value, a refresh a refresh of the value.

use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The change sets of `source` with each item as `factory` makes it.
pub(crate) fn transform_items<T: Clone + 'static, K: Eq + Hash + Clone + 'static, U: Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    factory: Rc<dyn Fn(&T, &K) -> U>,
) -> Rc<dyn IObservable<ChangeSet<U, K>>> {
    Observable::defer(move || {
        let cache = RefCell::new(ChangeAwareCache::<U, K>::new());
        let factory = factory.clone();
        source.select(move |changes: ChangeSet<T, K>| {
            for change in &changes {
                match change.reason() {
                    ChangeReason::Add | ChangeReason::Update => {
                        let transformed = factory(change.current(), change.key());
                        cache.borrow_mut().add_or_update(transformed, change.key().clone());
                    }
                    ChangeReason::Remove => cache.borrow_mut().remove(change.key()),
                    ChangeReason::Refresh => cache.borrow_mut().refresh(change.key()),
                    // Moves change nothing.
                    ChangeReason::Moved => {}
                }
            }
            let captured = cache.borrow_mut().capture_changes();
            captured
        })
    })
}

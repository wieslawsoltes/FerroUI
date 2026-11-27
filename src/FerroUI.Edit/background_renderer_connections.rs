use crate::{KnownLayer, TextArea};
use ferroui_base::WeakRef;
use std::cell::RefCell;

/// Shared, weak view bookkeeping for stateful built-in decorations. Connection
/// hooks may be repeated when the same renderer identity occurs in a collection.
#[derive(Default)]
pub(crate) struct BackgroundRendererConnections(RefCell<Vec<WeakRef<TextArea>>>);

impl BackgroundRendererConnections {
    pub(crate) fn add(&self, view: &TextArea) {
        let handle = view.to_ref();
        let mut views = self.0.borrow_mut();
        views.retain(|view| view.upgrade().is_some());
        if !views
            .iter()
            .any(|view| view.upgrade().as_ref() == Some(&handle))
        {
            views.push(handle.downgrade());
        }
    }

    pub(crate) fn remove(&self, view: &TextArea) {
        // TextArea invokes this hook from Drop too. Its weak identity remains
        // valid there, whereas constructing a strong to_ref() would panic.
        let handle = view.to_weak();
        self.0
            .borrow_mut()
            .retain(|view| !view.ptr_eq(&handle) && view.upgrade().is_some());
    }

    pub(crate) fn invalidate(&self, layer: KnownLayer) {
        // Native invalidation can call application code. Keep that code outside
        // the collection borrow, including when a renderer is shared by views.
        let views: Vec<_> = self
            .0
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
            .collect();
        for view in views {
            view.invalidate_layer(layer);
        }
    }
}

//! Not from the reference: the delivery of the results of the background threads of the file
//! system watcher, the network trigger and the HTTP request action to their objects on the thread
//! of the user interface.
//!
//! The reference raises the events of `FileSystemWatcher`, `NetworkChange` and `HttpClient` on
//! thread pool threads and posts the work to the dispatcher with a closure over the object. The
//! objects of the user interface belong to their thread here, so a background thread knows its
//! object by a number, and the job it posts finds the object by that number on the thread of the
//! user interface.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroObject, Ref, WeakRef};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

ferroui_base::ferro_thread_local! {
    static TARGETS: RefCell<Vec<(u64, WeakRef<FerroObject>)>> = const { RefCell::new(Vec::new()) };
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Registers `object` as the target of the results of a background operation; returns its number.
/// Called on the thread of the user interface.
pub(crate) fn register(object: &FerroObject) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let target = object.to_ref().downgrade();
    TARGETS.with(|targets| {
        let mut targets = targets.borrow_mut();
        targets.retain(|(_, target)| target.upgrade().is_some());
        targets.push((id, target));
    });
    id
}

/// Ends the delivery to the target `id`: the results that arrive later are dropped.
pub(crate) fn unregister(id: u64) {
    let _ = TARGETS.try_with(|targets| targets.borrow_mut().retain(|(target_id, _)| *target_id != id));
}

/// Posts `deliver` to `dispatcher`; it runs with the target `id` if the target is still registered
/// and alive. Called from any thread.
pub(crate) fn post_to(dispatcher: &Arc<Dispatcher>, id: u64, deliver: impl FnOnce(Ref<FerroObject>) + Send + 'static) {
    dispatcher.post(
        move || {
            let target = TARGETS.with(|targets| {
                targets.borrow().iter().find(|(target_id, _)| *target_id == id).and_then(|(_, target)| target.upgrade())
            });
            if let Some(target) = target {
                deliver(target);
            }
        },
        DispatcherPriority::DEFAULT,
    );
}

//! Remove keyboard callbacks without releasing user captures during SDK mutation.
use ferroui_system::events::HandlerList;

pub(crate) fn remove<F: ?Sized>(handlers: &HandlerList<F>, token: u64) -> bool {
    // The selected SDK removes under a RefCell borrow. Retaining its native
    // snapshot keeps captures alive until that borrow has ended. Destructors may
    // then subscribe, remove another callback, or panic without a nested borrow.
    let snapshot = handlers.snapshot();
    let removed = handlers.remove(token);
    drop(snapshot);
    removed
}

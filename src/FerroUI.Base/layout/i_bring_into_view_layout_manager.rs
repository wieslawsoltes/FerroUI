use super::{BringIntoViewRequest, ILayoutManager};
use std::rc::Rc;

/// An [`ILayoutManager`] that handles [`BringIntoViewRequest`]s.
pub trait IBringIntoViewLayoutManager: ILayoutManager {
    /// Gets whether a layout pass is currently running.
    fn is_in_layout_pass(&self) -> bool;

    /// Enqueues a bring-into-view request to be processed at the end of the
    /// current or next layout pass, replacing any previously enqueued request
    /// for the same target.
    fn enqueue_bring_into_view(&self, request: Rc<dyn BringIntoViewRequest>);
}
